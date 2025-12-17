use std::fs::{self};
use std::io::{Error, Write};
use std::process::{Command, Stdio};
use std::{env, path::PathBuf};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::lexer::tokens_generate;
use crate::navigation::{change_directory, get_current_working_dir};
use crate::parser::{Redirection, parse_tokens_to_args};
use crate::terminal_io::IoContext;

// 1. 重定向的操作模式 (对应 > 和 >>)

pub enum BuiltinCommand {
    ECHO,
    EXIT,
    TYPE,
    PWD,
    CD,
}
impl BuiltinCommand {
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinCommand::ECHO => "echo",
            BuiltinCommand::EXIT => "exit",
            BuiltinCommand::TYPE => "type",
            BuiltinCommand::PWD => "pwd",
            BuiltinCommand::CD => "cd",
        }
    }
}
pub enum MskCommand {
    Builtin(
        BuiltinCommand,
        Option<Vec<String>>,
        Option<Vec<Redirection>>,
    ),
    External(
        String,
        Vec<PathBuf>,
        Option<Vec<String>>,
        Option<Vec<Redirection>>,
    ),
    Unknown(String),
}
impl MskCommand {
    fn get_redirections(&self) -> &Option<Vec<Redirection>> {
        match self {
            MskCommand::Builtin(_, _, redirections) => redirections,
            MskCommand::External(_, _, _, redirections) => redirections,
            MskCommand::Unknown(_) => &None,
        }
    }
}
/// 也许这里可以传进String
pub fn parse_command(input: &str) -> Option<MskCommand> {
    let tokens = tokens_generate(input);

    let (mut all_parts, redirections) = parse_tokens_to_args(tokens);
    // println!("{:?}, {:?}\r", all_parts, redirections);
    // 3. 提取命令 (取出第一个)
    if all_parts.is_empty() {
        return None; // 输入只有空格或为空
    }

    // remove(0) 会移除并返回第一个元素，剩下的自动前移
    let cmd = all_parts.remove(0);
    let args = all_parts; // 剩下的就是参数列表

    match cmd.as_str() {
        "echo" => Some(MskCommand::Builtin(
            BuiltinCommand::ECHO,
            Some(args),
            redirections,
        )),
        "exit" => Some(MskCommand::Builtin(
            BuiltinCommand::EXIT,
            None,
            redirections,
        )),
        "type" => {
            if args.is_empty() {
                Some(MskCommand::Builtin(
                    BuiltinCommand::TYPE,
                    None,
                    redirections,
                ))
            } else {
                Some(MskCommand::Builtin(
                    BuiltinCommand::TYPE,
                    Some(args),
                    redirections,
                ))
            }
        }
        "pwd" => Some(MskCommand::Builtin(BuiltinCommand::PWD, None, redirections)),
        "cd" => {
            if args.is_empty() {
                Some(MskCommand::Builtin(BuiltinCommand::CD, None, redirections))
            } else {
                Some(MskCommand::Builtin(
                    BuiltinCommand::CD,
                    Some(args),
                    redirections,
                ))
            }
        }
        "" => None,
        other => {
            let env_path = env::var_os("PATH");
            if let Some(os_string) = env_path {
                let path_buf_iter = env::split_paths(&os_string);
                let executable_path = path_buf_iter
                    .map(|path| path.join(&cmd))
                    .filter(|candidate| is_executable(candidate))
                    .collect::<Vec<PathBuf>>();
                if !executable_path.is_empty() {
                    return Some(MskCommand::External(
                        other.to_string(),
                        executable_path,
                        Some(args),
                        redirections,
                    ));
                }
            }
            Some(MskCommand::Unknown(other.to_string()))
        }
    }
}
pub enum ProcessCmdError {
    IOError(Error),
    Other,
}
impl From<std::io::Error> for ProcessCmdError {
    fn from(e: std::io::Error) -> Self {
        ProcessCmdError::IOError(e)
    }
}
pub fn process_cmd(cmd: MskCommand) -> Result<(), ProcessCmdError> {
    let mut io_ctx = IoContext::new();
    let redirections_opt = cmd.get_redirections();
    if let Some(redirections) = redirections_opt {
        io_ctx.apply_redirections(redirections)?;
    }

    match cmd {
        MskCommand::Builtin(BuiltinCommand::ECHO, args, _) => {
            let mut writer = io_ctx.stdout.to_write();
            let output = args.unwrap().join(" ");
            write!(writer, "{}\r\n", output)?;
        }
        MskCommand::Builtin(BuiltinCommand::EXIT, _, _) => return Err(ProcessCmdError::Other),
        MskCommand::Builtin(BuiltinCommand::PWD, _, _) => {
            let mut writer = io_ctx.stdout.to_write();
            let pwd = get_current_working_dir();
            write!(writer, "{}\r\n", format!("{}", &pwd))?;
        }
        MskCommand::Builtin(BuiltinCommand::CD, args, _) => {
            if let Some(path) = args {
                change_directory(&path[0]);
            } else {
                change_directory("~");
            }
        }
        MskCommand::Builtin(BuiltinCommand::TYPE, args_opt, _) => {
            let msg = {
                if let Some(args) = args_opt {
                    match parse_command(&args[0]) {
                        None => unreachable!(),
                        Some(MskCommand::Builtin(command_type, _, _)) => {
                            format!("{} is a shell builtin", command_type.name())
                        }
                        Some(MskCommand::Unknown(name)) => {
                            format!("{}: not found", name)
                        }
                        Some(MskCommand::External(name, paths, _, _)) => {
                            format!("{} is {}", name, paths[0].to_string_lossy())
                        }
                    }
                } else {
                    "Usage: type <command>".to_string()
                }
            };

            let mut writer = io_ctx.stdout.to_write();
            write!(writer, "{}\r\n", format!("{}", &msg))?;
        }
        MskCommand::External(name, _paths, args, _) => {
            // terminal.flush();
            io_ctx.flush_stdout()?;
            let _ = disable_raw_mode();
            run_command(
                &name,
                args.as_deref(),
                io_ctx.stdout.to_stdio(),
                io_ctx.stderr.to_stdio(),
            );
            let _ = enable_raw_mode();
        }
        MskCommand::Unknown(name) => {
            let mut writer = io_ctx.stdout.to_write();
            write!(writer, "{}\r\n", format!("{}: command not found", &name))?;
        }
    }
    Ok(())
}

pub fn run_command(executable_file: &str, args_opt: Option<&[String]>, out: Stdio, err: Stdio) {
    let mut command = Command::new(executable_file);
    if let Some(args) = args_opt {
        command.args(args);
    }
    command.stdout(out);
    command.stderr(err);
    // status() 会启动子进程，阻塞当前线程直到子进程结束
    // 并且默认会继承父进程的 stdin/stdout/stderr (也就是直接打印到屏幕)
    match command.status() {
        Ok(exit_status) => {
            if exit_status.success() {
                // 成功运行且返回码为 0
            } else {
                // 运行了，但返回了非 0 错误码
                // 比如 grep 没找到东西返回 1
                // 你可以使用 exit_status.code() 获取具体数字
            }
        }
        Err(e) => {
            // 根本没跑起来（比如文件格式错误、IO错误等）
            eprintln!("Failed to execute command: {}", e);
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
