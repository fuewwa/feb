use std::collections::HashMap;
use std::fmt;

pub enum Value {
	Int(i64),
	Double(f64),
	Str(String),
}

impl fmt::Display for Value {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Value::Int(v) => write!(f, "{}", v),
			Value::Double(v) => write!(f, "{}", v),
			Value::Str(v) => write!(f, "{}", v),
		}
	}
}

pub struct Target {
	pub name: String,
	pub commands: Vec<String>,
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
	let mut in_quote = false;
	let mut prev_space = true;
	let mut chars = line.char_indices().peekable();
	while let Some((i, c)) = chars.next() {
		if c == '"' {
			in_quote = !in_quote;
		}
		if !in_quote && c == '/' && prev_space {
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

fn parse_value(text: &str) -> Value {
	let text = text.trim();
	if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
		return Value::Str(text[1..text.len() - 1].to_string());
	}
	if let Ok(v) = text.parse::<i64>() {
		return Value::Int(v);
	}
	if let Ok(v) = text.parse::<f64>() {
		return Value::Double(v);
	}
	Value::Str(text.to_string())
}

pub fn parse(text: &str) -> Result<Config, String> {
	let mut params = HashMap::new();
	let mut targets: Vec<Target> = Vec::new();
	let mut current: Option<Target> = None;

	for (index, raw) in text.lines().enumerate() {
		let number = index + 1;
		let raw = raw.trim_end_matches('\r');

		if let Some(target) = current.as_mut() {
			if raw.starts_with('\t') {
				let command = raw.trim();
				if !command.is_empty() && !command.starts_with("//") {
					target.commands.push(command.to_string());
				}
				continue;
			}
			let trimmed = raw.trim();
			if trimmed == "}" {
				targets.push(current.take().unwrap());
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
				return Err(format!("febconf:{}: invalid target name '{}'", number, name));
			}
			if targets.iter().any(|t| t.name == name) {
				return Err(format!("febconf:{}: duplicate target '{}'", number, name));
			}
			current = Some(Target {
				name: name.to_string(),
				commands: Vec::new(),
			});
			continue;
		}

		if let Some((key, value)) = trimmed.split_once('=') {
			let key = key.trim();
			if !valid_name(key) {
				return Err(format!("febconf:{}: invalid param name '{}'", number, key));
			}
			params.insert(key.to_string(), parse_value(value));
			continue;
		}

		return Err(format!("febconf:{}: cannot parse '{}'", number, trimmed));
	}

	if let Some(target) = current {
		return Err(format!("febconf: target '{}' is not closed with '}}'", target.name));
	}

	Ok(Config { params, targets })
}
