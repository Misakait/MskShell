use std::fs;
use std::{env, path::PathBuf};

pub enum BuiltinCommand {
    ECHO,
    EXIT,
    TYPE,
}
impl BuiltinCommand {
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinCommand::ECHO => "echo",
            BuiltinCommand::EXIT => "exit",
            BuiltinCommand::TYPE => "type",
        }
    }
}
pub enum MskCommand {
    Builtin(BuiltinCommand, Option<Vec<String>>),
    External(String, Vec<PathBuf>),
    Unknown(String),
}

/// 也许这里可以传进String
pub fn parse_command(input: &str) -> Option<MskCommand> {
    let mut parts = input.split_whitespace();
    let cmd = parts.next()?; // 如果没有 token 则返回 None (跳过空行)
    let args: Vec<String> = parts.map(|s| s.to_string()).collect();

    match cmd {
        "echo" => Some(MskCommand::Builtin(BuiltinCommand::ECHO, Some(args))),
        "exit" => Some(MskCommand::Builtin(BuiltinCommand::EXIT, None)),
        "type" => Some(MskCommand::Builtin(BuiltinCommand::TYPE, Some(args))),
        other => {
            let env_path = env::var_os("PATH");
            if let Some(os_string) = env_path {
                let path_buf_iter = env::split_paths(&os_string);
                let executable_path = path_buf_iter
                    .map(|path| path.join(cmd))
                    .filter(|candidate| is_executable(candidate))
                    .collect::<Vec<PathBuf>>();
                if !executable_path.is_empty() {
                    return Some(MskCommand::External(other.to_string(), executable_path));
                }
            }
            Some(MskCommand::Unknown(other.to_string()))
        }
    }
}

pub fn is_executable(path: &std::path::Path) -> bool {
    // 第一步：如果文件根本不存在，直接返回 false
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    // 第二步：必须是文件（目录虽然可能有 +x 权限，但不能执行）
    if !metadata.is_file() {
        return false;
    }

    // 第三步：根据系统判断权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 检查模式位：只要 owner, group, other 任何一方有执行权限(0o111)，就算可执行
        // 如果想严谨一点，只检查 owner (0o100) 也可以
        return metadata.permissions().mode() & 0o111 != 0;
    }

    #[cfg(windows)]
    {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let exec_exts = ["exe", "bat", "cmd", "com", "ps1", "msi"];
        return exec_exts.contains(&ext.to_lowercase().as_str());
    }

    // 对于其他非常见系统，默认返回 false
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}
