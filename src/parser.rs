use std::{env, panic, path::PathBuf};

use crate::lexer::{Args, Token};
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RedirectionMode {
    Overwrite, // >  (O_TRUNC)
    Append,    // >> (O_APPEND)
}

// 2. 重定向的目标 (将来可以支持 &1 这种 FD 重定向)
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectionTarget {
    File(PathBuf), // 文件路径
    Fd(i32),       // 文件描述符 (比如 2>&1) -> 这一步你可以先不实现，但位置留好
}

// 3. 单个重定向动作描述符
#[derive(Debug, Clone)]
pub struct Redirection {
    pub source_fd: i32,            // 谁要被重定向？(1=stdout, 2=stderr)
    pub target: RedirectionTarget, // 去哪里？
    pub mode: RedirectionMode,     // 怎么去？(覆盖还是追加)
}
pub fn parse_tokens_to_args(tokens: Vec<Token>) -> (Vec<String>, Option<Vec<Redirection>>) {
    let mut args = Vec::new();
    let mut redirections = Vec::new();
    let mut tokens_iter = tokens.into_iter();
    while let Some(token) = tokens_iter.next() {
        match token {
            // 这里以后要用正则或其他办法匹配形如100>的op
            Token::Op(op) => {
                let (source_fd, mode) = match op.as_str() {
                    ">" | "1>" => (1, RedirectionMode::Overwrite),
                    ">>" | "1>>" => (1, RedirectionMode::Append),
                    "2>" => (2, RedirectionMode::Overwrite),
                    "2>>" => (2, RedirectionMode::Append),
                    _ => todo!(),
                };
                let target_token = tokens_iter.next().expect("Redirect后面必须有参数");
                match target_token {
                    Token::Op(_) => panic!("Redirect后面参数只能是文件路径或文件描述符"),
                    Token::Word(items) => {
                        // TODO: target不会只是pathbuf
                        let redirection = Redirection {
                            source_fd,
                            target: RedirectionTarget::File(consolidate_args(items).into()),
                            mode,
                        };
                        redirections.push(redirection);
                    }
                };
            }
            Token::Word(items) => args.push(consolidate_args(items)),
        }
    }
    if redirections.is_empty() {
        (args, None)
    } else {
        (args, Some(redirections))
    }
}
fn consolidate_args(args: Vec<Args>) -> String {
    args.into_iter().map(|arg| expand_arg(arg)).collect()
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
    }
}
