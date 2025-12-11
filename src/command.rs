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
        other => Some(MskCommand::Unknown(other.to_string())),
    }
}
