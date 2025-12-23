use std::fs::{self};
use std::io::{Error, Write};
use std::num::ParseIntError;
use std::process::{Child, Command, exit};
use std::{env, path::PathBuf};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::lexer::{Token, tokens_generate};
use crate::navigation::{change_directory, get_current_working_dir};
use crate::parser::{Redirection, parse_tokens_to_args};
use crate::state::ShellState;
use crate::terminal_io::{InputStream, IoContext, OutputStream};

pub enum BuiltinCommand {
    ECHO,
    EXIT,
    HISTORY,
    TYPE,
    PWD,
    CD,
}
impl BuiltinCommand {
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinCommand::ECHO => "echo",
            BuiltinCommand::EXIT => "exit",
            BuiltinCommand::HISTORY => "history",
            BuiltinCommand::TYPE => "type",
            BuiltinCommand::PWD => "pwd",
            BuiltinCommand::CD => "cd",
        }
    }
}
pub struct Pipeline {
    pub commands: Vec<MskCommand>,
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
fn split_vec_by_sep<T: Eq>(vec: Vec<T>, sep: T) -> Vec<Vec<T>> {
    let mut result = Vec::new();
    let mut current_group = Vec::new();

    for item in vec {
        if item == sep {
            // 遇到分隔符：若当前组非空，存入结果并重置
            if !current_group.is_empty() {
                result.push(current_group);
                current_group = Vec::new();
            }
        } else {
            // 非分隔符：加入当前组
            current_group.push(item);
        }
    }

    // 遍历结束后，将最后一个非空组存入结果
    if !current_group.is_empty() {
        result.push(current_group);
    }

    result
}
pub fn parse_tokens_to_pipeline(tokens: Vec<Token>) -> Option<Pipeline> {
    let tokens_split = split_vec_by_sep(tokens, Token::Op("|".to_string()));
    let commands: Vec<MskCommand> = tokens_split
        .into_iter()
        .map(|v| parse_tokens_to_args(v))
        .map(|(all_parts, redirections)| parse_command(all_parts, redirections))
        .flatten()
        .collect();
    if commands.is_empty() {
        None
    } else {
        Some(Pipeline { commands })
    }
}
pub fn parse_input(input: &str) -> Option<Pipeline> {
    let tokens = tokens_generate(input);
    parse_tokens_to_pipeline(tokens)
}
// pub fn parse_command(input: &str) -> Option<MskCommand> {
pub fn parse_command(
    mut all_parts: Vec<String>,
    redirections: Option<Vec<Redirection>>,
) -> Option<MskCommand> {
    // let tokens = tokens_generate(input);

    // let (mut all_parts, redirections) = parse_tokens_to_args(tokens);
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
        "history" => {
            if args.is_empty() {
                Some(MskCommand::Builtin(
                    BuiltinCommand::HISTORY,
                    None,
                    redirections,
                ))
            } else {
                Some(MskCommand::Builtin(
                    BuiltinCommand::HISTORY,
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
#[derive(Debug)]
pub enum ProcessCmdError {
    IOError(Error),
    ArgsError(String),
    Other,
}
impl From<ParseIntError> for ProcessCmdError {
    fn from(_value: ParseIntError) -> Self {
        ProcessCmdError::ArgsError("该参数应为数字".into())
    }
}
impl From<std::io::Error> for ProcessCmdError {
    fn from(e: std::io::Error) -> Self {
        ProcessCmdError::IOError(e)
    }
}
pub fn run_pipeline(pipelne: Pipeline, state: &ShellState) -> Result<(), ProcessCmdError> {
    let _ = disable_raw_mode();
    let mut children: Vec<Child> = Vec::new();
    let mut previous_read_end = None;
    let mut first_cmd = true;
    let mut cmds = pipelne.commands.into_iter().peekable();
    while let Some(cmd) = cmds.next() {
        let io_ctx;
        if cmds.peek().is_some() {
            let (reader, writer) = std::io::pipe()?;
            // 如果后面有命令检查现在是不是第一条命令
            if first_cmd {
                // 第一条命令的输入就是系统，输出要给下一个命令当输入
                first_cmd = false;
                io_ctx = IoContext {
                    stdout: OutputStream::Pipe(writer),
                    stderr: OutputStream::Inherit,
                    stdin: InputStream::Inherit,
                };
            } else {
                // 下一条还有命令，但是自己不是第一条命令
                io_ctx = IoContext {
                    stdout: OutputStream::Pipe(writer),
                    stderr: OutputStream::Inherit,
                    // 此时可以安全unwrap因为第一次运行保证了里面必定有值
                    stdin: InputStream::Pipe(previous_read_end.take().unwrap()),
                };
            }
            // 给下一条命令保存读端
            previous_read_end = Some(reader);
        } else {
            if first_cmd {
                // 如果后面没有管道就证明自己是最后一条命令，直接写入标准输出
                // 但是如果如果自己同时是第一条命令，标准输入是继承
                io_ctx = IoContext::new()
            } else {
                io_ctx = IoContext {
                    stdout: OutputStream::Inherit,
                    stderr: OutputStream::Inherit,
                    // 此时可以安全unwrap因为第一次运行保证了里面必定有值
                    stdin: InputStream::Pipe(previous_read_end.take().unwrap()),
                };
            }
        }

        match process_single_cmd(cmd, io_ctx, state) {
            Ok(Some(child)) => children.push(child),
            Ok(None) => {} // Builtin 命令没有子进程
            Err(e) => eprintln!("Command execution error: {:?}\r", e),
        }
    }
    for mut child in children {
        let _ = child.wait();
    }
    let _ = enable_raw_mode();
    Ok(())
}
pub fn process_single_cmd(
    cmd: MskCommand,
    mut io_ctx: IoContext,
    state: &ShellState,
) -> Result<Option<Child>, ProcessCmdError> {
    // let mut cmds = pipelne.commands.into_iter().peekable();
    // let mut io_ctx = IoContext::new();
    // io_ctx.stdin = stdin;
    // io_ctx.stdout = OutputStream::from_stdio(stdout);
    // io_ctx.stderr = OutputStream::from_stdio(stderr);
    // while let Some(cmd) = cmds.next() {
    let redirections_opt = cmd.get_redirections();
    if let Some(redirections) = redirections_opt {
        io_ctx.apply_redirections(redirections)?;
    }

    match cmd {
        MskCommand::Builtin(BuiltinCommand::ECHO, args, _) => {
            let mut writer = io_ctx.stdout.to_write();
            let output = args.unwrap().join(" ");
            write!(writer, "{}\n", output)?;
        }
        MskCommand::Builtin(BuiltinCommand::EXIT, _, _) => exit(0),
        MskCommand::Builtin(BuiltinCommand::PWD, _, _) => {
            let mut writer = io_ctx.stdout.to_write();
            let pwd = get_current_working_dir();
            write!(writer, "{}\n", format!("{}", &pwd))?;
        }
        MskCommand::Builtin(BuiltinCommand::CD, args, _) => {
            if let Some(path) = args {
                change_directory(&path[0]);
            } else {
                change_directory("~");
            }
        }
        MskCommand::Builtin(BuiltinCommand::HISTORY, args_opt, _) => {
            let mut writer = io_ctx.stdout.to_write();
            if let Some(args) = args_opt {
                let limit = args[0].parse::<usize>()?;
                let history_len = state.history.len();
                // 若 limit >= 历史总数，从 0 开始；否则从 history_len - limit 开始
                let start_idx = history_len.saturating_sub(limit);
                for (idx, command) in state.history[start_idx..].iter().enumerate() {
                    let display_idx = start_idx + idx + 1;
                    writeln!(writer, "{:5}  {}", display_idx, command)?;
                }
            } else {
                for (i, command) in state.history.iter().enumerate() {
                    writeln!(writer, "{:5}  {}", i + 1, command)?;
                }
            }
        }
        MskCommand::Builtin(BuiltinCommand::TYPE, args_opt, _) => {
            let msg = {
                if let Some(args) = args_opt {
                    // match parse_command(&args[0]) {
                    match parse_command(args, None) {
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
            write!(writer, "{}\n", format!("{}", &msg))?;
        }
        MskCommand::External(name, _paths, args, _) => {
            // terminal.flush();
            io_ctx.flush_stdout()?;
            // let _ = disable_raw_mode();
            // run_command(
            //     &name,
            //     args.as_deref(),
            //     io_ctx.stdout.to_stdio(),
            //     io_ctx.stderr.to_stdio(),
            // );
            let mut command = Command::new(name);
            if let Some(a) = args {
                command.args(a);
            }

            let child = command
                .stdin(io_ctx.stdin.to_stdio())
                .stdout(io_ctx.stdout.to_stdio())
                .stderr(io_ctx.stderr.to_stdio())
                .spawn()?;

            // let _ = enable_raw_mode();
            return Ok(Some(child));
        }
        MskCommand::Unknown(name) => {
            let mut writer = io_ctx.stdout.to_write();
            write!(writer, "{}\n", format!("{}: command not found", &name))?;
        }
    }
    // }
    Ok(None)
}

// pub fn run_command(executable_file: &str, args_opt: Option<&[String]>, out: Stdio, err: Stdio) {
//     let mut command = Command::new(executable_file);
//     if let Some(args) = args_opt {
//         command.args(args);
//     }
//     command.stdout(out);
//     command.stderr(err);
//     // status() 会启动子进程，阻塞当前线程直到子进程结束
//     // 并且默认会继承父进程的 stdin/stdout/stderr (也就是直接打印到屏幕)
//     match command.status() {
//         Ok(exit_status) => {
//             if exit_status.success() {
//                 // 成功运行且返回码为 0
//             } else {
//                 // 运行了，但返回了非 0 错误码
//                 // 比如 grep 没找到东西返回 1
//                 // 你可以使用 exit_status.code() 获取具体数字
//             }
//         }
//         Err(e) => {
//             // 根本没跑起来（比如文件格式错误、IO错误等）
//             eprintln!("Failed to execute command: {}", e);
//         }
//     }
// }
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
