pub enum MskEvent {
    Key(MskKeyCode),
}
pub enum MskKeyCode {
    Char(char),
    Backspace,
    Enter,
    ArrowRight,
    ArrowLeft,
    Tab,
    Up,
    Down,
}

pub fn get_event() -> Option<MskEvent> {
    use crossterm::event::{Event, KeyCode, KeyModifiers, read};
    match read() {
        Ok(Event::Key(key_event)) => match key_event.code {
            KeyCode::Char('j') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                return Some(MskEvent::Key(MskKeyCode::Enter));
            }
            KeyCode::Char(c) => {
                return Some(MskEvent::Key(MskKeyCode::Char(c)));
            }
            KeyCode::Backspace => return Some(MskEvent::Key(MskKeyCode::Backspace)),
            KeyCode::Enter => return Some(MskEvent::Key(MskKeyCode::Enter)),
            KeyCode::Right => return Some(MskEvent::Key(MskKeyCode::ArrowRight)),
            KeyCode::Left => return Some(MskEvent::Key(MskKeyCode::ArrowLeft)),
            KeyCode::Tab => return Some(MskEvent::Key(MskKeyCode::Tab)),
            KeyCode::Up => return Some(MskEvent::Key(MskKeyCode::Up)),
            KeyCode::Down => return Some(MskEvent::Key(MskKeyCode::Down)),
            _ => {
                return None;
            }
        },
        // 处理 Resize 等其他事件，忽略并继续等
        Ok(_) => return None,
        Err(_) => return None,
    }
}
use std::fs::{File, OpenOptions};
use std::io::{self, PipeReader, PipeWriter, Write};
use std::process::Stdio;

use crate::parser::{Redirection, RedirectionMode, RedirectionTarget};

// 定义输出流的目标：要么是继承父进程（屏幕），要么是文件
// 将来支持管道时，这里加一个 Pipe
pub enum OutputStream {
    Inherit,    // 默认：屏幕
    File(File), // 重定向：文件
    Pipe(PipeWriter),
}

impl OutputStream {
    pub fn to_stdio(self) -> Stdio {
        match self {
            OutputStream::Inherit => Stdio::inherit(),
            OutputStream::File(f) => Stdio::from(f.try_clone().unwrap()),
            OutputStream::Pipe(stdio) => stdio.into(),
        }
    }

    pub fn to_write(&mut self) -> Box<dyn Write + '_> {
        match self {
            OutputStream::Inherit => Box::new(io::stdout()),
            OutputStream::File(f) => Box::new(f),
            OutputStream::Pipe(stdio) => Box::new(stdio),
        }
    }
}
pub enum InputStream {
    Inherit,
    Pipe(PipeReader),
}
impl InputStream {
    pub fn to_stdio(self) -> Stdio {
        match self {
            InputStream::Inherit => Stdio::inherit(),
            InputStream::Pipe(stdio) => stdio.into(),
        }
    }
}
// I/O 上下文：管理当前命令的 stdin/stdout/stderr
pub struct IoContext {
    pub stdout: OutputStream,
    pub stderr: OutputStream,
    pub stdin: InputStream,
}

impl IoContext {
    pub fn new() -> Self {
        Self {
            stdout: OutputStream::Inherit,
            stderr: OutputStream::Inherit,
            stdin: InputStream::Inherit,
        }
    }
    pub fn flush_stdout(&mut self) -> io::Result<()> {
        match &mut self.stdout {
            // 如果是 Inherit，说明指向的是标准输出，刷新 io::stdout
            OutputStream::Inherit => io::stdout().flush()?,
            // 如果是 File，调用 File 的 flush (系统调用 fsync 或类似)
            OutputStream::File(f) => f.flush()?,
            OutputStream::Pipe(stdio) => stdio.flush()?,
        }

        Ok(())
    }
    pub fn flush_stderr(&mut self) -> io::Result<()> {
        match &mut self.stderr {
            // 如果是 Inherit，说明指向的是标准错误，刷新 io::stderr
            OutputStream::Inherit => io::stderr().flush()?,
            OutputStream::File(f) => f.flush()?,
            OutputStream::Pipe(stdio) => stdio.flush()?,
        }

        Ok(())
    }
    // 核心逻辑：根据重定向列表，修改上下文
    // 这一步是把 "Configuration" 变成 "Runtime Resources"
    pub fn apply_redirections(&mut self, redirections: &[Redirection]) -> io::Result<()> {
        for r in redirections {
            // 1. 打开文件
            let mut opts = OpenOptions::new();
            opts.write(true).create(true);
            match r.mode {
                RedirectionMode::Overwrite => {
                    opts.truncate(true);
                }
                RedirectionMode::Append => {
                    opts.append(true);
                }
            }

            if let RedirectionTarget::File(path) = &r.target {
                let file = opts.open(path)?;

                // 2. 替换流
                match r.source_fd {
                    1 => self.stdout = OutputStream::File(file),
                    2 => self.stderr = OutputStream::File(file),
                    _ => {} // 暂不支持其他 fd
                }
            }
        }
        Ok(())
    }
}
