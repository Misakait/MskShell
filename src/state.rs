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

    pub fn add_history(&mut self, command: String) {
        if !command.trim().is_empty() {
            self.history.push(command.to_string());
        }
    }
}
