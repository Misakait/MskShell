use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self};

pub struct RawModeGuard;

impl RawModeGuard {
    pub fn new() -> Result<Self, io::Error> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // 无论如何退出（Panic或正常结束），都要恢复终端！
        let _ = disable_raw_mode();
    }
}
