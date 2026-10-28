use std::collections::HashMap;
use std::fmt;

pub enum Value {
	Bool(bool),
	Int(i64),
	Double(f64),
	Str(String),
}

impl Value {
	pub fn kind(&self) -> &'static str {
		match self {
			Value::Bool(_) => "bool",
			Value::Int(_) => "int",
			Value::Double(_) => "double",
			Value::Str(_) => "string",
		}
	}
}

impl fmt::Display for Value {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Value::Bool(v) => write!(f, "{}", v),
			Value::Int(v) => write!(f, "{}", v),
			Value::Double(v) => write!(f, "{}", v),
			Value::Str(v) => write!(f, "{}", v),
		}
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum Op {
	Eq,
	Ne,
	Gt,
	Lt,
	Ge,
	Le,
}

impl Op {
	pub fn symbol(&self) -> &'static str {
		match self {
			Op::Eq => "==",
			Op::Ne => "!=",
			Op::Gt => ">=",
			Op::Lt => "<=",
			Op::Ge => ">!",
			Op::Le => "<!",
		}
	}
}

pub enum Operand {
	Lit(Value),
	Param(String),
}

pub struct Cond {
	pub left: Operand,
	pub op: Op,
	pub right: Operand,
	pub line: usize,
}

pub enum Node {
	Cmd(String),
	If {
		cond: Cond,
		then: Vec<Node>,
		other: Vec<Node>,
	},
}

pub struct Target {
	pub name: String,
	pub nodes: Vec<Node>,
}

pub struct Config {
	pub params: HashMap<String, Value>,
	pub targets: Vec<Target>,
}

impl Config {
	pub fn target(&self, name: &str) -> Option<&Target> {
		self.targets.iter().find(|t| t.name == name)
	}
}

fn strip_comment(line: &str) -> &str {
	let mut quote: Option<char> = None;
	let mut prev_space = true;
	let mut chars = line.char_indices().peekable();
	while let Some((i, c)) = chars.next() {
		match quote {
			Some(q) if c == q => quote = None,
			None if c == '"' || c == '\'' => quote = Some(c),
			_ => {}
		}
		if quote.is_none() && c == '/' && prev_space {
			if let Some(&(_, '/')) = chars.peek() {
				return &line[..i];
			}
		}
		prev_space = c.is_whitespace();
	}
	line
}

fn valid_name(name: &str) -> bool {
	!name.is_empty()
		&& name
			.chars()
			.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn looks_numeric(text: &str) -> bool {
	let first = text.chars().next().unwrap_or(' ');
	let last = text.chars().last().unwrap_or(' ');
	(first.is_ascii_digit() || first == '-' || first == '+' || first == '.')
		&& (last.is_ascii_digit() || last == '.')
}

fn parse_value(text: &str) -> Result<Value, String> {
	let text = text.trim();
	let first = match text.chars().next() {
		Some(c) => c,
		None => return Err("missing value".to_string()),
	};
	if first == '"' || first == '\'' {
		let inner = &text[1..];
		if let Some(body) = inner.strip_suffix(first) {
			if !body.contains(first) {
				return Ok(Value::Str(body.to_string()));
			}
		}
		return Err(format!("malformed string {}", text));
	}
	match text {
		"true" => return Ok(Value::Bool(true)),
		"false" => return Ok(Value::Bool(false)),
		_ => {}
	}
	if looks_numeric(text) {
		if let Ok(v) = text.parse::<i64>() {
			return Ok(Value::Int(v));
		}
		if let Ok(v) = text.parse::<f64>() {
			return Ok(Value::Double(v));
		}
	}
	Err(format!(
		"'{}' is not a bool, int, double or quoted string",
		text
	))
}

fn find_op(text: &str) -> Option<(usize, Op)> {
	let chars: Vec<(usize, char)> = text.char_indices().collect();
	let mut quote: Option<char> = None;
	for k in 0..chars.len() {
		let (i, c) = chars[k];
		if let Some(q) = quote {
			if c == q {
				quote = None;
			}
			continue;
		}
		if c == '"' || c == '\'' {
			quote = Some(c);
			continue;
		}
		if k + 1 < chars.len() {
			let op = match (c, chars[k + 1].1) {
				('=', '=') => Op::Eq,
				('!', '=') => Op::Ne,
				('>', '=') => Op::Gt,
				('<', '=') => Op::Lt,
				('>', '!') => Op::Ge,
				('<', '!') => Op::Le,
				_ => continue,
			};
			return Some((i, op));
		}
	}
	None
}

fn parse_operand(text: &str) -> Result<Operand, String> {
	let text = text.trim();
	let first = match text.chars().next() {
		Some(c) => c,
		None => return Err("missing operand".to_string()),
	};
	if first == '"' || first == '\'' || text == "true" || text == "false" || looks_numeric(text) {
		return parse_value(text).map(Operand::Lit);
	}
	if valid_name(text) {
		return Ok(Operand::Param(text.to_string()));
	}
	Err(format!("'{}' is not a param name or a value", text))
}

fn parse_cond(text: &str, line: usize) -> Result<Cond, String> {
	let (at, op) =
		find_op(text).ok_or_else(|| "a condition needs one of == != >= <= >! <!".to_string())?;
	let left = parse_operand(&text[..at])?;
	let right = parse_operand(&text[at + 2..])?;
	Ok(Cond {
		left,
		op,
		right,
		line,
	})
}

struct Frame {
	cond: Cond,
	then: Vec<Node>,
	other: Vec<Node>,
	in_else: bool,
}

struct Build {
	name: String,
	root: Vec<Node>,
	stack: Vec<Frame>,
}

impl Build {
	fn push(&mut self, node: Node) {
		match self.stack.last_mut() {
			Some(frame) if frame.in_else => frame.other.push(node),
			Some(frame) => frame.then.push(node),
			None => self.root.push(node),
		}
	}
}

fn is_if(text: &str) -> bool {
	(text.starts_with("if ") || text.starts_with("if\t")) && text.ends_with(':')
}

fn parse_body_line(build: &mut Build, raw: &str, number: usize) -> Result<(), String> {
	let depth = raw.chars().take_while(|c| *c == '\t').count();
	let text = raw[depth..].trim();
	if text.is_empty() || text.starts_with("//") {
		return Ok(());
	}
	let level = build.stack.len();
	let at = |expected: usize| -> Result<(), String> {
		if depth == expected {
			Ok(())
		} else {
			Err(format!(
				"febconf:{}: expected {} TAB character(s) here, found {}",
				number, expected, depth
			))
		}
	};

	if is_if(text) {
		at(level + 1)?;
		let head = &text[2..text.len() - 1];
		let cond = parse_cond(head, number).map_err(|e| format!("febconf:{}: {}", number, e))?;
		build.stack.push(Frame {
			cond,
			then: Vec::new(),
			other: Vec::new(),
			in_else: false,
		});
	} else if text == "else" {
		if level == 0 {
			return Err(format!("febconf:{}: 'else' without 'if'", number));
		}
		at(level)?;
		let frame = build.stack.last_mut().unwrap();
		if frame.in_else {
			return Err(format!("febconf:{}: second 'else' in one 'if'", number));
		}
		frame.in_else = true;
	} else if text == "fi" {
		if level == 0 {
			return Err(format!("febconf:{}: 'fi' without 'if'", number));
		}
		at(level)?;
		let frame = build.stack.pop().unwrap();
		build.push(Node::If {
			cond: frame.cond,
			then: frame.then,
			other: frame.other,
		});
	} else {
		at(level + 1)?;
		build.push(Node::Cmd(text.to_string()));
	}
	Ok(())
}

pub fn parse(text: &str) -> Result<Config, String> {
	let mut params = HashMap::new();
	let mut targets: Vec<Target> = Vec::new();
	let mut current: Option<Build> = None;

	for (index, raw) in text.lines().enumerate() {
		let number = index + 1;
		let raw = raw.trim_end_matches('\r');

		if let Some(build) = current.as_mut() {
			if raw.starts_with('\t') {
				parse_body_line(build, raw, number)?;
				continue;
			}
			let trimmed = raw.trim();
			if trimmed == "}" {
				let build = current.take().unwrap();
				if !build.stack.is_empty() {
					return Err(format!(
						"febconf:{}: 'if' without 'fi' in target '{}'",
						number, build.name
					));
				}
				targets.push(Target {
					name: build.name,
					nodes: build.root,
				});
				continue;
			}
			if trimmed.is_empty() || trimmed.starts_with("//") {
				continue;
			}
			return Err(format!(
				"febconf:{}: command lines must start with a TAB character",
				number
			));
		}

		let trimmed = strip_comment(raw).trim();
		if trimmed.is_empty() {
			continue;
		}

		if let Some(head) = trimmed.strip_suffix('{') {
			let name = head.trim();
			if !valid_name(name) {
				return Err(format!(
					"febconf:{}: invalid target name '{}'",
					number, name
				));
			}
			if targets.iter().any(|t| t.name == name) {
				return Err(format!("febconf:{}: duplicate target '{}'", number, name));
			}
			current = Some(Build {
				name: name.to_string(),
				root: Vec::new(),
				stack: Vec::new(),
			});
			continue;
		}

		if let Some((key, value)) = trimmed.split_once('=') {
			let key = key.trim();
			if !valid_name(key) {
				return Err(format!("febconf:{}: invalid param name '{}'", number, key));
			}
			let value = parse_value(value).map_err(|e| format!("febconf:{}: {}", number, e))?;
			params.insert(key.to_string(), value);
			continue;
		}

		return Err(format!("febconf:{}: cannot parse '{}'", number, trimmed));
	}

	if let Some(build) = current {
		return Err(format!(
			"febconf: target '{}' is not closed with '}}'",
			build.name
		));
	}

	Ok(Config { params, targets })
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn nested_if_parses() {
		let text = "a = 1\nt {\n\tif a == 1:\n\t\tif a != 2:\n\t\t\techo x\n\t\tfi\n\telse\n\t\techo y\n\tfi\n}\n";
		assert!(parse(text).is_ok());
	}

	#[test]
	fn body_needs_extra_tab() {
		let text = "t {\n\tif a == 1:\n\techo x\n\tfi\n}\n";
		assert!(parse(text).err().unwrap().contains("expected 2"));
	}

	#[test]
	fn missing_fi_is_an_error() {
		let text = "t {\n\tif a == 1:\n\t\techo x\n}\n";
		assert!(parse(text).err().unwrap().contains("without 'fi'"));
	}

	#[test]
	fn stray_else_and_fi_are_errors() {
		assert!(parse("t {\n\telse\n}\n").is_err());
		assert!(parse("t {\n\tfi\n}\n").is_err());
	}

	#[test]
	fn colon_is_required() {
		assert!(parse("t {\n\tif a == 1\n\t\techo x\n\tfi\n}\n").is_err());
	}

	#[test]
	fn operators_are_mapped() {
		assert!(matches!(find_op("a == b"), Some((2, Op::Eq))));
		assert!(matches!(find_op("a != b"), Some((2, Op::Ne))));
		assert!(matches!(find_op("a >= b"), Some((2, Op::Gt))));
		assert!(matches!(find_op("a <= b"), Some((2, Op::Lt))));
		assert!(matches!(find_op("a >! b"), Some((2, Op::Ge))));
		assert!(matches!(find_op("a <! b"), Some((2, Op::Le))));
	}

	#[test]
	fn operator_in_quotes_is_ignored() {
		assert!(matches!(find_op("\"a==b\" == x"), Some((7, Op::Eq))));
	}
}
