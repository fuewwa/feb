use crate::parser::{Cond, Config, Node, Op, Operand, Target, Value};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::process::Command;

fn expand(command: &str, params: &HashMap<String, Value>) -> Result<String, String> {
	let mut out = String::new();
	let mut rest = command;
	while let Some(start) = rest.find("${") {
		out.push_str(&rest[..start]);
		let after = &rest[start + 2..];
		let end = after
			.find('}')
			.ok_or_else(|| format!("unclosed '${{' in '{}'", command))?;
		let name = after[..end].trim();
		let value = params
			.get(name)
			.ok_or_else(|| format!("undefined param '{}' in '{}'", name, command))?;
		out.push_str(&value.to_string());
		rest = &after[end + 1..];
	}
	out.push_str(rest);
	Ok(out)
}

fn shell(line: &str) -> Command {
	if cfg!(windows) {
		let mut cmd = Command::new("cmd");
		cmd.arg("/C").arg(line);
		cmd
	} else {
		let mut cmd = Command::new("sh");
		cmd.arg("-c").arg(line);
		cmd
	}
}

fn number(value: &Value) -> Option<f64> {
	match value {
		Value::Int(v) => Some(*v as f64),
		Value::Double(v) => Some(*v),
		_ => None,
	}
}

fn ordering(left: &Value, right: &Value) -> Option<Ordering> {
	match (left, right) {
		(Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
		_ => number(left)?.partial_cmp(&number(right)?),
	}
}

fn compare(op: Op, left: &Value, right: &Value) -> Result<bool, String> {
	match op {
		Op::Eq | Op::Ne => {
			let equal = match (left, right) {
				(Value::Bool(a), Value::Bool(b)) => a == b,
				(Value::Str(a), Value::Str(b)) => a == b,
				_ => match ordering(left, right) {
					Some(order) => order == Ordering::Equal,
					None => {
						return Err(format!(
							"cannot compare {} with {} using '{}'",
							left.kind(),
							right.kind(),
							op.symbol()
						))
					}
				},
			};
			Ok(equal == (op == Op::Eq))
		}
		_ => {
			let order = ordering(left, right).ok_or_else(|| {
				format!(
					"'{}' needs two numbers, got {} and {}",
					op.symbol(),
					left.kind(),
					right.kind()
				)
			})?;
			Ok(match op {
				Op::Gt => order == Ordering::Greater,
				Op::Lt => order == Ordering::Less,
				Op::Ge => order != Ordering::Less,
				_ => order != Ordering::Greater,
			})
		}
	}
}

fn resolve<'a>(
	operand: &'a Operand,
	params: &'a HashMap<String, Value>,
) -> Result<&'a Value, String> {
	match operand {
		Operand::Lit(value) => Ok(value),
		Operand::Param(name) => params
			.get(name)
			.ok_or_else(|| format!("undefined param '{}'", name)),
	}
}

fn holds(cond: &Cond, params: &HashMap<String, Value>) -> Result<bool, String> {
	let wrap = |e: String| format!("febconf:{}: {}", cond.line, e);
	let left = resolve(&cond.left, params).map_err(wrap)?;
	let right = resolve(&cond.right, params).map_err(wrap)?;
	compare(cond.op, left, right).map_err(wrap)
}

fn run_command(command: &str, params: &HashMap<String, Value>) -> Result<(), String> {
	let line = expand(command, params)?;
	println!("{}", line);
	let status = shell(&line)
		.status()
		.map_err(|e| format!("cannot start '{}': {}", line, e))?;
	if !status.success() {
		return Err(match status.code() {
			Some(code) => format!("'{}' exited with code {}", line, code),
			None => format!("'{}' was terminated by a signal", line),
		});
	}
	Ok(())
}

fn run_nodes(nodes: &[Node], params: &HashMap<String, Value>) -> Result<(), String> {
	for node in nodes {
		match node {
			Node::Cmd(command) => run_command(command, params)?,
			Node::If { cond, then, other } => {
				let branch = if holds(cond, params)? { then } else { other };
				run_nodes(branch, params)?;
			}
		}
	}
	Ok(())
}

pub fn run(config: &Config, target: &Target) -> Result<(), String> {
	run_nodes(&target.nodes, &config.params)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn gt_is_strict_and_ge_is_not() {
		let three = Value::Int(3);
		assert!(!compare(Op::Gt, &three, &Value::Int(3)).unwrap());
		assert!(compare(Op::Ge, &three, &Value::Int(3)).unwrap());
		assert!(compare(Op::Gt, &three, &Value::Int(2)).unwrap());
	}

	#[test]
	fn lt_is_strict_and_le_is_not() {
		let three = Value::Int(3);
		assert!(!compare(Op::Lt, &three, &Value::Int(3)).unwrap());
		assert!(compare(Op::Le, &three, &Value::Int(3)).unwrap());
		assert!(compare(Op::Lt, &three, &Value::Int(4)).unwrap());
	}

	#[test]
	fn int_and_double_compare_by_number() {
		assert!(compare(Op::Eq, &Value::Int(2), &Value::Double(2.0)).unwrap());
		assert!(compare(Op::Lt, &Value::Double(1.5), &Value::Int(2)).unwrap());
	}

	#[test]
	fn strings_and_bools_support_eq_and_ne() {
		let a = Value::Str("py".to_string());
		let b = Value::Str("cpp".to_string());
		assert!(compare(Op::Ne, &a, &b).unwrap());
		assert!(compare(Op::Eq, &Value::Bool(true), &Value::Bool(true)).unwrap());
	}

	#[test]
	fn mixed_types_are_errors() {
		let text = Value::Str("1".to_string());
		assert!(compare(Op::Eq, &text, &Value::Int(1)).is_err());
		assert!(compare(Op::Gt, &text, &Value::Str("0".to_string())).is_err());
		assert!(compare(Op::Gt, &Value::Bool(true), &Value::Int(0)).is_err());
	}
}
