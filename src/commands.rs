pub enum Command {
    Exit,
    Clear,
    Models,
    Usage,
    SwitchModel(String),
    SystemPrompt,
    Chat(String),
}

pub fn parse_command(input: &str) -> Command {
    match input {
        "/exit" => Command::Exit,
        "/clear" => Command::Clear,
        "/models" => Command::Models,
        "/usage" => Command::Usage,
        "/system" => Command::SystemPrompt,
        s if s.starts_with("/model ") => {
            Command::SwitchModel(s.strip_prefix("/model ").unwrap().trim().to_string())
        }
          
        s => Command::Chat(s.to_string()),
    }
}

/// List of available commands, used for Tab autocomplete.
pub const COMMAND_LIST: [&str; 6] = ["/exit", "/clear", "/models", "/usage", "/model ", "/system"];
