pub struct ShellState {
    pub history: Vec<String>,
    pub history_cursor: usize,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            history_cursor: 0,
        }
    }

    pub fn add_history(&mut self, command: String) {
        if !command.trim().is_empty() {
            self.history.push(command.to_string());
        }
    }
}
