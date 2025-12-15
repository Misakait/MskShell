use crate::terminal_io::{MskEvent, MskKeyCode, TerminalIO};

pub struct LineEditor {
    buffer: Vec<char>, // 存的是完整的字符
    cursor: usize,
    // utf8_buf: Vec<u8>, // 暂存还没收全的字节
}

impl LineEditor {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            cursor: 0,
            // utf8_buf: Vec::with_capacity(4), // UTF-8 最多4个字节
        }
    }

    // 第一层：接收原始字节（供中断或 main loop 调用）
    // pub fn handle_byte(&mut self, byte: u8, terminal: &mut impl TerminalIO) -> Option<String> {
    //     self.utf8_buf.push(byte);

    //     // 尝试把 utf8_buf 转换成 str
    //     // core::str::from_utf8 会检查字节序列是否完整且合法
    //     match core::str::from_utf8(&self.utf8_buf) {
    //         Ok(s) => {
    //             // 成功！凑出了一个合法的字符（可能是 'a'，也可能是 '你'）
    //             let c = s.chars().next().unwrap();

    //             // 清空缓存，准备接收下一个字
    //             self.utf8_buf.clear();

    //             // 交给第二层逻辑处理
    //             return self.handle_char(c, terminal);
    //         }
    //         Err(_) => {
    //             // 转换失败。有两种可能：
    //             // 1. 字节还没收全 (e.error_len() 为 None) -> 啥也不做，等下一个字节
    //             // 2. 也是乱码 -> 这里可以做容错处理，清空 buffer

    //             // 简单的做法：如果 buffer 满了(4字节)还解不出来，说明是废数据，清空重来
    //             if self.utf8_buf.len() >= 4 {
    //                 self.utf8_buf.clear();
    //             }
    //             None
    //         }
    //     }
    // }
    pub fn handle_event(
        &mut self,
        event: MskEvent,
        terminal: &mut impl TerminalIO,
    ) -> Option<String> {
        match event {
            MskEvent::Key(msk_key_code) => match msk_key_code {
                MskKeyCode::Char(c) => self.handle_char(c, terminal),
                MskKeyCode::Backspace => self.handle_backsapce(terminal),
                MskKeyCode::Enter => self.handle_return(terminal),
                MskKeyCode::ArrowRight => self.handle_arrow_right(terminal),
                MskKeyCode::ArrowLeft => self.handle_arrow_left(terminal),
            },
        }
    }
    fn handle_arrow_right(&mut self, terminal: &mut impl TerminalIO) -> Option<String> {
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
            terminal.write_str("\x1b[C");
        }
        None
    }
    fn handle_arrow_left(&mut self, terminal: &mut impl TerminalIO) -> Option<String> {
        if self.cursor > 0 {
            self.cursor -= 1;
            terminal.write_str("\x1b[D");
        }
        None
    }
    fn handle_backsapce(&mut self, terminal: &mut impl TerminalIO) -> Option<String> {
        if self.cursor > 0 {
            self.buffer.remove(self.cursor - 1);
            self.cursor -= 1;
            terminal.write_str("\x08\x1b[P");
        }
        None
    }
    fn handle_return(&mut self, terminal: &mut impl TerminalIO) -> Option<String> {
        let line: String = self.buffer.iter().collect();
        self.buffer.clear();
        self.cursor = 0;
        terminal.write_str("\r\n");
        Some(line)
    }
    // TODO: 以后支持中文逻辑
    fn handle_char(&mut self, c: char, terminal: &mut impl TerminalIO) -> Option<String> {
        if self.cursor == self.buffer.len() {
            // 插入字符
            self.buffer.insert(self.cursor, c);
            self.cursor += 1;

            // 回显逻辑也要支持中文！
            // 注意：这里回显不能只 write_byte，要 write_str
            let mut temp_buf = [0u8; 4];
            let s = c.encode_utf8(&mut temp_buf);
            terminal.write_str(s);
        } else {
            terminal.write_str("\x1b[@");
            self.buffer.insert(self.cursor, c);
            self.cursor += 1;

            let mut temp_buf = [0u8; 4];
            let s = c.encode_utf8(&mut temp_buf);
            terminal.write_str(s);
        }
        None
    }
}
