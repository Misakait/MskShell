use std::fs;
use std::process::Command;
use std::{env, path::PathBuf};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::navigation::{change_directory, get_current_working_dir};
use crate::terminal_io::TerminalIO;

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
    Builtin(BuiltinCommand, Option<Vec<String>>),
    External(String, Vec<PathBuf>, Option<Vec<String>>),
    Unknown(String),
}
#[derive(Debug)]
pub enum Args {
    Raw(String),
    SingleQuotes(String),
    DoubleQuotes(String),
    Split,
}

// 这个函数把 [Raw("a"), Single("b"), Split, Double("c")]
// 变成 ["ab", "c"]
fn consolidate_args(args: Vec<Args>) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_arg = String::new();
    let mut is_building_arg = false; // 标记当前是否正在构建一个参数

    for arg in args {
        match arg {
            Args::Split => {
                // 遇到分隔符，说明上一个参数结束了
                if is_building_arg {
                    result.push(current_arg);
                    current_arg = String::new();
                    is_building_arg = false;
                }
            }
            _ => {
                let part = expand_arg(arg);

                // 2. 拼接到当前参数后面
                current_arg.push_str(&part);
                is_building_arg = true;
            }
        }
    }

    // 别忘了把最后一个参数塞进去
    if is_building_arg {
        result.push(current_arg);
    }

    result
}
/// 也许这里可以传进String
pub fn parse_command(input: &str) -> Option<MskCommand> {
    let (cmd, args) = parse_input_to_args(input);
    let args = consolidate_args(args);
    // args.into_iter().map(|arg| arg.into_string())
    // let mut parts = input.split_whitespace();
    // let cmd = parts.next()?; // 如果没有 token 则返回 None (跳过空行)
    // let args: Vec<String> = parts.map(|s| s.to_string()).collect();

    match cmd.as_str() {
        "echo" => Some(MskCommand::Builtin(BuiltinCommand::ECHO, Some(args))),
        "exit" => Some(MskCommand::Builtin(BuiltinCommand::EXIT, None)),
        "type" => {
            if args.is_empty() {
                Some(MskCommand::Builtin(BuiltinCommand::TYPE, None))
            } else {
                Some(MskCommand::Builtin(BuiltinCommand::TYPE, Some(args)))
            }
        }
        "pwd" => Some(MskCommand::Builtin(BuiltinCommand::PWD, None)),
        "cd" => {
            if args.is_empty() {
                Some(MskCommand::Builtin(BuiltinCommand::CD, None))
            } else {
                Some(MskCommand::Builtin(BuiltinCommand::CD, Some(args)))
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
                    ));
                }
            }
            Some(MskCommand::Unknown(other.to_string()))
        }
    }
}

pub fn parse_input_to_args(input: &str) -> (String, Vec<Args>) {
    let mut args: Vec<Args> = Vec::new();
    let mut cmd = String::new();
    let mut input_iter = input.chars().peekable();
    while let Some(c) = input_iter.next()
        && !c.is_whitespace()
    {
        cmd.push(c);
    }

    'outer: while let Some(char) = input_iter.next() {
        let mut str = String::new();
        match char {
            '\"' => {
                while let Some(c) = input_iter.next() {
                    match c {
                        '\"' => {
                            args.push(Args::DoubleQuotes(str));
                            break;
                        }
                        _ => {
                            str.push(c);
                        }
                    }
                }
            }
            '\'' => {
                while let Some(c) = input_iter.next() {
                    // println!("single quotes");
                    match c {
                        '\'' => {
                            args.push(Args::SingleQuotes(str));
                            break;
                        }
                        _ => {
                            str.push(c);
                        }
                    }
                }
            }
            _ => {
                // 如果这是一个空格并且前面是引号参数，直接push一个空白分割
                if matches!(
                    args.last(),
                    Some(Args::DoubleQuotes(_) | Args::SingleQuotes(_))
                ) && char.is_whitespace()
                {
                    args.push(Args::Split);
                } else {
                    str.push(char);
                }
                while let Some(c) = input_iter.peek() {
                    match c {
                        '\'' | '\"' => {
                            let mut iter_clone = input_iter.clone();
                            let next = iter_clone.next();
                            let next_next = iter_clone.next();
                            // Empty quotes are ignored.
                            if next == next_next {
                                input_iter.next();
                                input_iter.next();
                            } else {
                                // 前提条件，下一个参数将是引号参数
                                // 第一种情况，前面是正常字符串带个空白字符结尾，
                                // 先插入本身，然后插入分割
                                if str.trim_end().len() != str.len() && !str.trim().is_empty() {
                                    args.push(Args::Raw(str.trim().to_string()));
                                    args.push(Args::Split);
                                }
                                // 前面是正常字符串，这里不可能是空白字符
                                // 空白字符去除后面空白字符之后长度不相等
                                if str.trim_end().len() == str.len() {
                                    args.push(Args::Raw(str.trim().to_string()));
                                }
                                // 前面是空白字符
                                if str.trim().is_empty() {
                                    args.push(Args::Split);
                                }
                                continue 'outer;
                            }
                        }
                        _ => {
                            let c = input_iter.next().unwrap();
                            if c.is_whitespace() && !str.trim().is_empty() {
                                args.push(Args::Raw(str.trim().to_string()));
                                args.push(Args::Split); //maybe delete
                                continue 'outer;
                            }
                            str.push(c);
                        }
                    }
                }
                // print!("here");
                if !str.trim().is_empty() {
                    args.push(Args::Raw(str.trim().to_string()));
                    break;
                }
            }
        }
    }
    //如果最后是一个分隔符去掉
    if matches!(args.last(), Some(Args::Split)) {
        args.pop();
    }
    (cmd, args)
}
pub fn process_cmd(cmd: MskCommand, terminal: &mut impl TerminalIO) -> Result<(), ()> {
    match cmd {
        MskCommand::Builtin(BuiltinCommand::ECHO, args) => {
            terminal.write_str(&format!("{}", args.unwrap().join(" ")));
            terminal.write_str("\r\n");
        }
        MskCommand::Builtin(BuiltinCommand::EXIT, _) => return Err(()),
        MskCommand::Builtin(BuiltinCommand::PWD, _) => {
            let pwd = get_current_working_dir();
            terminal.write_str(&pwd);
            terminal.write_str("\r\n");
        }
        MskCommand::Builtin(BuiltinCommand::CD, args) => {
            if let Some(path) = args {
                change_directory(&path[0]);
            } else {
                change_directory("~");
            }
        }
        MskCommand::Builtin(BuiltinCommand::TYPE, args_opt) => {
            let msg = {
                if let Some(args) = args_opt {
                    match parse_command(&args[0]) {
                        None => unreachable!(),
                        Some(MskCommand::Builtin(command_type, _)) => {
                            format!("{} is a shell builtin", command_type.name())
                        }
                        Some(MskCommand::Unknown(name)) => {
                            format!("{}: not found", name)
                        }
                        Some(MskCommand::External(name, paths, _)) => {
                            format!("{} is {}", name, paths[0].to_string_lossy())
                        }
                    }
                } else {
                    "Usage: type <command>".to_string()
                }
            };

            terminal.write_str(&msg);
            terminal.write_str("\r\n");
        }
        MskCommand::External(name, _paths, args) => {
            terminal.flush();
            let _ = disable_raw_mode();
            run_command(&name, args.as_deref());
            let _ = enable_raw_mode();
        }
        MskCommand::Unknown(name) => {
            terminal.write_str(&name);
            terminal.write_str(": command not found\r\n");
        }
    }
    Ok(())
}
fn expand_arg(arg: Args) -> String {
    match arg {
        Args::Raw(s) => {
            if s == "~" {
                env::var("HOME").unwrap_or_else(|_| s.clone())
            } else if s.starts_with("~/") {
                if let Ok(home) = env::var("HOME") {
                    // 拼接: /home/user + /Downloads
                    format!("{}{}", home, &s[1..])
                } else {
                    s
                }
            } else {
                s
            }
        }
        Args::SingleQuotes(s) => s,
        Args::DoubleQuotes(s) => s,
        Args::Split => "".to_string(),
    }
}
pub fn run_command(executable_file: &str, args_opt: Option<&[String]>) {
    let mut command = Command::new(executable_file);
    if let Some(args) = args_opt {
        command.args(args);
    }

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
