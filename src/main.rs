#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        let mut command = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).unwrap();
        match command.trim() {
            _ => println!("{}: command not found", command.trim()),
        }
    }
}
