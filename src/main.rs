#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    print!("$ ");
<<<<<<< HEAD
    let mut command;
=======
    let mut command = String::new();
>>>>>>> 5dbe443 (Read user input and report unknown commands)
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut command).unwrap();
    match command.trim() {
        _ => println!("{}: command not found", command.trim()),
    }
}
