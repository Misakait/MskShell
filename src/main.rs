#[allow(unused_imports)]
use std::io::{self, Write};

use crate::command::{BuiltinCommand, MskCommand, parse_command};
mod command;
fn main() {
    loop {
        print!("$ ");
        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let cmd_opt = parse_command(&input);
        let cmd = match cmd_opt {
            None => continue, // 空行，继续读取下一行
            Some(c) => c,
        };
        match cmd {
            MskCommand::Builtin(BuiltinCommand::ECHO, args) => {
                println!("{}", args.unwrap().join(" "));
            }
            MskCommand::Builtin(BuiltinCommand::EXIT, _) => break,
            MskCommand::Builtin(BuiltinCommand::TYPE, args) => {
                let args = args.unwrap();
                match parse_command(&args[0]) {
                    None => {
                        println!("Usage: type <command>");
                    }
                    Some(MskCommand::Builtin(command_type, _)) => {
                        println!("{} is a shell builtin", command_type.name());
                    }
                    Some(MskCommand::Unknown(name)) => {
                        println!("{}: not found", name);
                    }
                    Some(MskCommand::External(name, paths)) => {
                        println!("{} is {}", name, paths[0].to_string_lossy());
                    }
                };
            }
            MskCommand::External(_, _) => {}
            MskCommand::Unknown(name) => {
                println!("{}: command not found", name);
            }
        }
    }
}
