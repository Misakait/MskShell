use std::{env, fs};

pub struct ShellState {
    pub history: Vec<String>,
    pub history_cursor: usize,
    pub history_written_count: usize,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            history_cursor: 0,
            history_written_count: 0,
        }
    }

    pub fn init(&mut self) -> Result<(), std::io::Error> {
        if let Ok(path) = env::var("HISTFILE") {
            if let Ok(history_commands) = fs::read_to_string(path) {
                let mut cmds: Vec<String> = history_commands
                    .lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();
                self.history.append(&mut cmds);
            }
        }
        Ok(())
    }

    pub fn add_history(&mut self, command: String) {
        if !command.trim().is_empty() {
            self.history.push(command.to_string());
        }
    }
}
