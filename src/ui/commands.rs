pub enum Command {
    Exit,
    Clear,
    Models,
    SwitchModel(String),
    SystemPrompt,
    Chat(String),
    Skills,
    InvokeSkill(String),
}

pub fn parse_command(input: &str) -> Command {
    match input {
        "/exit" => Command::Exit,
        "/clear" => Command::Clear,
        "/models" => Command::Models,
        "/system" => Command::SystemPrompt,
        "/skills" => Command::Skills,
        s if s.starts_with("/model ") => {
            Command::SwitchModel(s.strip_prefix("/model ").unwrap().trim().to_string())
        }
        s if s.starts_with("/skill ") => {
            Command::InvokeSkill(s.strip_prefix("/skill ").unwrap().trim().to_string())
        }

        s => Command::Chat(s.to_string()),
    }
}

/// List of available commands, used for Tab autocomplete.
pub const COMMAND_LIST: [&str; 8] = [
    "/exit", "/clear", "/models", "/usage", "/model ", "/system", "/skills ", "skill ",
];
