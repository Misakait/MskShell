pub enum MskEvent {
    Key(MskKeyCode),
}
pub enum MskKeyCode {
    Char(char),
    Backspace,
    Enter,
    ArrowRight,
    ArrowLeft,
}
pub trait TerminalIO {
    fn write_byte(&mut self, byte: u8);

    fn write_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b);
        }
    }

    fn flush(&mut self);

    // fn read_byte(&mut self) -> Option<u8>;
    fn get_event(&mut self) -> Option<MskEvent>;
}

#[cfg(feature = "std")]
pub struct StdioTerminal {
    stdout: std::io::Stdout,
    // pending_bytes: VecDeque<u8>,
}

#[cfg(feature = "std")]
impl StdioTerminal {
    pub fn new() -> Self {
        Self {
            stdout: std::io::stdout(),
            // pending_bytes: VecDeque::new(),
        }
    }
}

#[cfg(feature = "std")]
impl TerminalIO for StdioTerminal {
    fn write_byte(&mut self, byte: u8) {
        use std::io::Write;
        let _ = self.stdout.write(&[byte]);
    }

    fn flush(&mut self) {
        use std::io::Write;
        let _ = self.stdout.flush();
    }
    fn get_event(&mut self) -> Option<MskEvent> {
        // // 1. 如果缓冲区有存货，直接返回（非阻塞）
        // if let Some(b) = self.pending_bytes.pop_front() {
        //     return Some(MskEvent::Key(MskKeyCode::Char(b)));
        // }
        use crossterm::event::{Event, KeyCode, read};
        match read() {
            Ok(Event::Key(key_event)) => match key_event.code {
                KeyCode::Char(c) => {
                    // let mut buf = [0; 4];
                    // let s = c.encode_utf8(&mut buf);
                    // for b in s.bytes() {
                    //     self.pending_bytes.push_back(b);
                    // }
                    return Some(MskEvent::Key(MskKeyCode::Char(c)));
                }
                KeyCode::Backspace => return Some(MskEvent::Key(MskKeyCode::Backspace)),
                KeyCode::Enter => return Some(MskEvent::Key(MskKeyCode::Enter)),
                KeyCode::Right => return Some(MskEvent::Key(MskKeyCode::ArrowRight)),
                KeyCode::Left => return Some(MskEvent::Key(MskKeyCode::ArrowLeft)),
                _ => {
                    return None;
                }
            },
            // 处理 Resize 等其他事件，忽略并继续等
            Ok(_) => return None,
            Err(_) => return None,
        }
    }
}
