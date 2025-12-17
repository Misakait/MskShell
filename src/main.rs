use crate::terminal_io::get_event;
use crate::{
    command::{parse_command, process_cmd},
    line_editor::LineEditor,
    raw_mode_guard::RawModeGuard,
};

use std::io::{self, Write};

mod command;
mod lexer;
mod line_editor;
mod navigation;
mod parser;
mod raw_mode_guard;
mod terminal_io;
fn main() -> Result<(), io::Error> {
    let _raw_guard = RawModeGuard::new()?;
    // let mut terminal = StdioTerminal::new();
    let mut editor = LineEditor::new();
    write!(io::stdout(), "$ ")?;
    io::stdout().flush()?;
    loop {
        if let Some(event) = get_event() {
            if let Some(input) = editor.handle_event(event) {
                let cmd_opt = parse_command(&input);
                let cmd = match cmd_opt {
                    None => {
                        write!(io::stdout(), "\r")?;
                        write!(io::stdout(), "$ ")?;
                        io::stdout().flush()?;
                        continue;
                    } // 空行，继续读取下一行
                    Some(c) => c,
                };
                if let Err(_) = process_cmd(cmd) {
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
