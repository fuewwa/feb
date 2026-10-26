use crate::parser::{Config, Target, Value};
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

pub fn run(config: &Config, target: &Target) -> Result<(), String> {
	for command in &target.commands {
		let line = expand(command, &config.params)?;
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
	}
	Ok(())
}
