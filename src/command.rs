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
#[derive(Debug, PartialEq)]
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
    let raw_tokens = parse_input_to_args(input);

    // 合并、展开、处理转义 (Merge & Expand)
    // 这一步之后，我们得到一个纯净的字符串列表
    // 例如: ["my program", "arg1", "arg2"]
    let mut all_parts = consolidate_args(raw_tokens);

    // 3. 提取命令 (取出第一个)
    if all_parts.is_empty() {
        return None; // 输入只有空格或为空
    }

    // remove(0) 会移除并返回第一个元素，剩下的自动前移
    let cmd = all_parts.remove(0);
    let args = all_parts; // 剩下的就是参数列表
    // println!("{:?}", args);
    // let args = consolidate_args(all_args);
    // println!("{:?}", args);
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
#[derive(PartialEq)]
enum ParseState {
    Normal,         // 正在读取普通字符 (Raw)
    InSingleQuotes, // 正在读取单引号内容
    InDoubleQuotes, // 正在读取双引号内容
}
pub fn parse_input_to_args(input: &str) -> Vec<Args> {
    let mut args: Vec<Args> = Vec::new();

    // 1. 提取命令名 (保持你原来的逻辑，遇到空白停止)
    let mut chars = input.chars().peekable();
    // while let Some(&c) = chars.peek() {
    //     if c.is_whitespace() {
    //         break;
    //     }
    //     cmd.push(chars.next().unwrap());
    // }

    // 2. 状态机解析参数
    let mut current_token = String::new();
    let mut state = ParseState::Normal;

    // 只要有字符就继续循环
    while let Some(c) = chars.next() {
        match state {
            // === 状态 1: 普通模式 (Raw) ===
            ParseState::Normal => {
                match c {
                    // 处理转义：反斜杠在 Raw 模式下意味着“下一个字符就是字面意思”
                    '\\' => {
                        if let Some(next_char) = chars.next() {
                            current_token.push(next_char);
                        }
                    }
                    // 遇到单引号 -> 切换状态
                    '\'' => {
                        // 如果之前有积攒的 Raw 字符，先保存
                        if !current_token.is_empty() {
                            args.push(Args::Raw(current_token.clone()));
                            current_token.clear();
                        }
                        state = ParseState::InSingleQuotes;
                    }
                    // 遇到双引号 -> 切换状态
                    '\"' => {
                        if !current_token.is_empty() {
                            args.push(Args::Raw(current_token.clone()));
                            current_token.clear();
                        }
                        state = ParseState::InDoubleQuotes;
                    }
                    // 遇到空格 -> 这是一个分隔符
                    c if c.is_whitespace() => {
                        if !current_token.is_empty() {
                            args.push(Args::Raw(current_token.clone()));
                            current_token.clear();
                        }
                        // 只有当上一个不是 Split 时才 push，避免重复 Split
                        if !matches!(args.last(), Some(Args::Split)) {
                            args.push(Args::Split);
                        }
                    }
                    // 普通字符 -> 放入 buffer
                    _ => {
                        current_token.push(c);
                    }
                }
            }

            // === 状态 2: 单引号模式 (绝对字面量) ===
            ParseState::InSingleQuotes => {
                match c {
                    // 遇到单引号 -> 结束
                    '\'' => {
                        args.push(Args::SingleQuotes(current_token.clone()));
                        current_token.clear();
                        state = ParseState::Normal;
                    }
                    // 其他任何字符（包括 \）都原样保存
                    _ => current_token.push(c),
                }
            }

            // === 状态 3: 双引号模式 (部分转义) ===
            ParseState::InDoubleQuotes => {
                match c {
                    // 遇到双引号 -> 结束
                    '\"' => {
                        args.push(Args::DoubleQuotes(current_token.clone()));
                        current_token.clear();
                        state = ParseState::Normal;
                    }
                    // 处理转义：在双引号里，\ 只有后面跟特定字符时才算转义
                    '\\' => {
                        // 偷看下一个字符
                        match chars.peek() {
                            // 如果后面是 \ " $ 或换行，由于这些在双引号里有特殊含义，
                            // 所以这里的 \ 起到了转义作用 -> 吃掉 \，保留后面的字符
                            Some(&'\\') | Some(&'\"') | Some(&'$') | Some(&'\n') | Some(&'`') => {
                                current_token.push(chars.next().unwrap());
                            }
                            // 如果后面是普通字符 (比如 \a)，Bash 的规则是保留 \
                            // 即 "\a" -> "\a"
                            _ => {
                                current_token.push('\\');
                            }
                        }
                    }
                    // 普通字符
                    _ => current_token.push(c),
                }
            }
        }
    }

    // 循环结束后的清理工作
    // 如果最后还有剩下的 Raw 字符
    if !current_token.is_empty() {
        if state == ParseState::Normal {
            args.push(Args::Raw(current_token));
        } else {
            // 如果还在引号状态里循环就结束了，说明引号未闭合
            // 这里可以报错，或者暂且当做 Raw 处理
        }
    }

    // 清理末尾多余的 Split
    if matches!(args.last(), Some(Args::Split)) {
        args.pop();
    }
    // 清理开头多余的 Split
    if !args.is_empty() && matches!(args[0], Args::Split) {
        args.remove(0);
    }
    args
}
// pub fn parse_input_to_args(input: &str) -> (String, Vec<Args>) {
//     let mut args: Vec<Args> = Vec::new();
//     let mut cmd = String::new();
//     let mut input_iter = input.chars().peekable();
//     while let Some(c) = input_iter.next()
//         && !c.is_whitespace()
//     {
//         cmd.push(c);
//     }

//     'outer: while let Some(char) = input_iter.next() {
//         let mut str = String::new();
//         match char {
//             '\"' => {
//                 while let Some(c) = input_iter.next() {
//                     match c {
//                         '\"' => {
//                             args.push(Args::DoubleQuotes(str));
//                             break;
//                         }
//                         _ => {
//                             str.push(c);
//                         }
//                     }
//                 }
//             }
//             '\'' => {
//                 while let Some(c) = input_iter.next() {
//                     // println!("single quotes");
//                     match c {
//                         '\'' => {
//                             args.push(Args::SingleQuotes(str));
//                             break;
//                         }
//                         _ => {
//                             str.push(c);
//                         }
//                     }
//                 }
//             }
//             _ => {
//                 // 如果这是一个空格并且前面是引号参数，直接push一个空白分割
//                 if matches!(
//                     args.last(),
//                     Some(Args::DoubleQuotes(_) | Args::SingleQuotes(_))
//                 ) && char.is_whitespace()
//                 {
//                     args.push(Args::Split);
//                 }
//                 // else {
//                 if char == '\\' {
//                     match input_iter.next() {
//                         Some(c) => str.push(c),
//                         None => {}
//                     }
//                     match input_iter.peek() {
//                         Some(_c) => {}
//                         None => args.push(Args::Raw(str.to_string())),
//                     }
//                 } else {
//                     str.push(char);
//                 }
//                 // }
//                 while let Some(c) = input_iter.peek() {
//                     match c {
//                         '\\' => {
//                             // 直接吞下\
//                             // let c = input_iter.next().unwrap();
//                             // str.push(c);
//                             input_iter.next().unwrap();
//                             // 吞下下一个字符
//                             match input_iter.next() {
//                                 Some(c) => str.push(c),
//                                 None => args.push(Args::Raw(str.trim().to_string())),
//                             }
//                         }
//                         '\'' | '\"' => {
//                             let mut iter_clone = input_iter.clone();
//                             let next = iter_clone.next();
//                             let next_next = iter_clone.next();
//                             // Empty quotes are ignored.
//                             if next == next_next {
//                                 input_iter.next();
//                                 input_iter.next();
//                             } else {
//                                 // 前提条件，下一个参数将是引号参数
//                                 // 第一种情况，前面是正常字符串带个空白字符结尾，
//                                 // 先插入本身，然后插入分割
//                                 if str.trim_end().len() != str.len() && !str.trim().is_empty() {
//                                     args.push(Args::Raw(str.trim().to_string()));
//                                     args.push(Args::Split);
//                                 }
//                                 // 前面是正常字符串，这里不可能是空白字符
//                                 // 空白字符去除后面空白字符之后长度不相等
//                                 if str.trim_end().len() == str.len() {
//                                     args.push(Args::Raw(str.trim().to_string()));
//                                 }
//                                 // 前面是空白字符
//                                 if str.trim().is_empty() {
//                                     args.push(Args::Split);
//                                 }
//                                 continue 'outer;
//                             }
//                         }
//                         _ => {
//                             let c = input_iter.next().unwrap();
//                             if c.is_whitespace() && !str.trim().is_empty() {
//                                 args.push(Args::Raw(str.trim().to_string()));
//                                 args.push(Args::Split); //maybe delete
//                                 continue 'outer;
//                             }
//                             str.push(c);
//                         }
//                     }
//                 }
//                 // print!("here");
//                 if !str.trim().is_empty() {
//                     args.push(Args::Raw(str.trim().to_string()));
//                     break;
//                 }
//             }
//         }
//     }
//     //如果最后是一个分隔符去掉
//     if matches!(args.last(), Some(Args::Split)) {
//         args.pop();
//     }
//     (cmd, args)
// }
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
        Args::Split => panic!("Split shoud be handled in consolidation phase"),
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
