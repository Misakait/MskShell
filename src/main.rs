#[cfg(feature = "std")]
use crate::terminal_io::StdioTerminal;
use crate::{
    command::{parse_command, process_cmd},
    line_editor::LineEditor,
    raw_mode_guard::RawModeGuard,
    terminal_io::TerminalIO,
};
#[allow(unused_imports)]
use std::io::{self, Write};

mod command;
mod line_editor;
mod navigation;
mod raw_mode_guard;
mod terminal_io;
fn main() -> Result<(), io::Error> {
    let _raw_guard = RawModeGuard::new()?;
    #[cfg(feature = "std")]
    let mut terminal = StdioTerminal::new();
    let mut editor = LineEditor::new();
    terminal.write_str("$ ");
    terminal.flush();
    loop {
        if let Some(event) = terminal.get_event() {
            if let Some(input) = editor.handle_event(event, &mut terminal) {
                let cmd_opt = parse_command(&input);
                let cmd = match cmd_opt {
                    None => {
                        terminal.write_str("\r");
                        terminal.write_str("$ ");
                        terminal.flush();
                        continue;
                    } // 空行，继续读取下一行
                    Some(c) => c,
                };
                if let Err(_) = process_cmd(cmd, &mut terminal) {
                    break;
                }
                terminal.write_str("\r");
                terminal.write_str("$ ");
            }

            // 每处理完一个字节，刷新一下缓冲区,保证回显输出
            terminal.flush();
        }
    }
    Ok(())
}
