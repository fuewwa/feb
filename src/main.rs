mod parser;
mod runner;

use std::env;
use std::fs;
use std::process;

const CONFIG_NAME: &str = "febconf";

fn usage() {
    println!("feb {} - fast & easy build", env!("CARGO_PKG_VERSION"));
    println!();
    println!("usage: feb [target ...]");
    println!("       feb --list");
    println!("       feb --help");
    println!("       feb --version");
    println!();
    println!("without a target the first target in {} is built", CONFIG_NAME);
}

fn fail(message: &str) -> ! {
    eprintln!("feb: error: {}", message);
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        return;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("feb {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let text = match fs::read_to_string(CONFIG_NAME) {
        Ok(text) => text,
        Err(e) => fail(&format!("cannot read {}: {}", CONFIG_NAME, e)),
    };
    let config = match parser::parse(&text) {
        Ok(config) => config,
        Err(e) => fail(&e),
    };

    if args.iter().any(|a| a == "-l" || a == "--list") {
        for target in &config.targets {
            println!("{}", target.name);
        }
        return;
    }

    let names: Vec<String> = if args.is_empty() {
        match config.targets.first() {
            Some(target) => vec![target.name.clone()],
            None => fail(&format!("no targets defined in {}", CONFIG_NAME)),
        }
    } else {
        args
    };

    for name in &names {
        if config.target(name).is_none() {
            let known: Vec<&str> = config.targets.iter().map(|t| t.name.as_str()).collect();
            fail(&format!(
                "unknown target '{}' (available: {})",
                name,
                known.join(", ")
            ));
        }
    }

    for name in &names {
        let target = config.target(name).unwrap();
        if let Err(e) = runner::run(&config, target) {
            fail(&format!("target '{}' failed: {}", name, e));
        }
    }
}
