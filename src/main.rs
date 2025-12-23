use crate::autocompletion::collect_all_commands;
use crate::command::{parse_input, run_pipeline};
use crate::state::ShellState;
use crate::terminal_io::get_event;
use crate::{line_editor::LineEditor, raw_mode_guard::RawModeGuard};

use std::io::{self, Write};

mod autocompletion;
mod command;
mod lexer;
mod line_editor;
mod navigation;
mod parser;
mod raw_mode_guard;
mod state;
mod terminal_io;
mod trie;

fn main() -> Result<(), io::Error> {
    let _raw_guard = RawModeGuard::new()?;
    let all_commands = collect_all_commands();
    let mut editor = LineEditor::new();
    let mut state = ShellState::new();
    write!(io::stdout(), "$ ")?;
    io::stdout().flush()?;
    loop {
        if let Some(event) = get_event() {
            if let Some(input) = editor.handle_event(event, &all_commands) {
                let cmd_opt = parse_input(&input);
                let cmd;
                match cmd_opt {
                    None => {
                        // 空行，继续读取下一行vv
                        write!(io::stdout(), "\r")?;
                        write!(io::stdout(), "$ ")?;
                        io::stdout().flush()?;
                        continue;
                    }
                    Some(c) => {
                        cmd = c;
                        state.add_history(input);
                    }
                };
                if let Err(_) = run_pipeline(cmd, &state) {
                    break;
                }
                write!(io::stdout(), "\r")?;
                write!(io::stdout(), "$ ")?;
            }

            // 每处理完一个字节，刷新一下缓冲区,保证回显输出
            io::stdout().flush()?;
        }
    }
    Ok(())
}
