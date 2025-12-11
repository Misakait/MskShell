#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let mut args = input.split_whitespace();
        let command = args.next().unwrap();
        match command {
            "echo" => {
                let content: String = args.collect::<Vec<&str>>().join(" ");
                println!("{}", content);
            }
            "exit" => break,
            _ => println!("{}: command not found", command.trim()),
        }
    }
}
