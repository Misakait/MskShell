#[cfg(target_os = "linux")]
use std::collections::VecDeque;

pub trait TerminalIO {
    fn write_byte(&mut self, byte: u8);

    fn write_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b);
        }
    }

    fn flush(&mut self);

    fn read_byte(&mut self) -> Option<u8>;
}

#[cfg(feature = "std")]
pub struct StdioTerminal {
    stdout: std::io::Stdout,
    pending_bytes: VecDeque<u8>,
}

#[cfg(feature = "std")]
impl StdioTerminal {
    pub fn new() -> Self {
        Self {
            stdout: std::io::stdout(),
            pending_bytes: VecDeque::new(),
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
    fn read_byte(&mut self) -> Option<u8> {
        // 1. 如果缓冲区有存货，直接返回（非阻塞）
        if let Some(b) = self.pending_bytes.pop_front() {
            return Some(b);
        }
        use crossterm::event::{Event, KeyCode, read};
        match read() {
            Ok(Event::Key(key_event)) => match key_event.code {
                KeyCode::Char(c) => {
                    let mut buf = [0; 4];
                    let s = c.encode_utf8(&mut buf);
                    for b in s.bytes() {
                        self.pending_bytes.push_back(b);
                    }
                    return self.pending_bytes.pop_front();
                }
                KeyCode::Backspace => return Some(0x08),
                KeyCode::Enter => return Some(b'\n'),
                _ => {
                    return None;
                }
            },
            // 处理 Resize 等其他事件，忽略并继续等
            Ok(_) => return None,
            Err(_) => return None,
        }
    }
    // fn read_byte(&mut self) -> Option<u8> {
    //     if let Some(b) = self.pending_bytes.pop_front() {
    //         return Some(b);
    //     }
    //     use crossterm::event::poll;
    //     use std::time::Duration;
    //     // 1. 询问：有没有数据准备好了？
    //     // Duration::from_secs(0) 表示完全不等待，立即返回结果
    //     match poll(Duration::from_secs(0)) {
    //         Ok(true) => {
    //             use crossterm::event::{Event, read};
    //             // 2. 有数据！因为 poll 说有了，所以这里的 read 肯定瞬间完成，不会阻塞
    //             if let Ok(Event::Key(key_event)) = read() {
    //                 use crossterm::event::KeyCode;
    //                 match key_event.code {
    //                     KeyCode::Char(c) => {
    //                         let mut buf = [0; 4];
    //                         let s = c.encode_utf8(&mut buf); // s 是 "你" (3 bytes)
    //                         // 把这些字节全部塞进队列
    //                         for b in s.bytes() {
    //                             self.pending_bytes.push_back(b);
    //                         }
    //                         // 立即返回第一个字节
    //                         return self.pending_bytes.pop_front();
    //                     }
    //                     KeyCode::Backspace => return Some(0x08),
    //                     KeyCode::Enter => return Some(b'\n'),
    //                     _ => return None,
    //                 }
    //             }
    //         }
    //         Ok(false) => {
    //             // 3. 没数据。不要阻塞，直接返回 None
    //             return None;
    //         }
    //         Err(_) => return None,
    //     }
    //     None
    // }
}
