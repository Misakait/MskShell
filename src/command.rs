pub enum BuiltinCommand {
    ECHO,
    EXIT,
}
pub enum MskCommand {
    Builtin(BuiltinCommand, Vec<String>),
    Unknown(String),
}

/// 也许这里可以传进String
pub fn parse_command(input: &str) -> Option<MskCommand> {
    let mut parts = input.split_whitespace();
    let cmd = parts.next()?; // 如果没有 token 则返回 None (跳过空行)
    let args: Vec<String> = parts.map(|s| s.to_string()).collect();

    match cmd {
        "echo" => Some(MskCommand::Builtin(BuiltinCommand::ECHO, args)),
        "exit" => Some(MskCommand::Builtin(BuiltinCommand::EXIT, args)),
        other => Some(MskCommand::Unknown(other.to_string())),
    }
}
