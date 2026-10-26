use std::collections::HashMap;
use std::fmt;

pub enum Value {
	Bool(bool),
	Int(i64),
	Double(f64),
	Str(String),
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
			let value = parse_value(value).map_err(|e| format!("febconf:{}: {}", number, e))?;
			params.insert(key.to_string(), value);
			continue;
		}

		return Err(format!("febconf:{}: cannot parse '{}'", number, trimmed));
	}

	if let Some(target) = current {
		return Err(format!("febconf: target '{}' is not closed with '}}'", target.name));
	}

	Ok(Config { params, targets })
}
