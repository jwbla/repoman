use crate::agent;
use crate::config::Config;
use crate::error::{RepomanError, Result};

pub fn handle_agent(action: &str, config: &Config) -> Result<()> {
    match action {
        "start" => {
            agent::start_agent(config)?;
            println!("Agent started");
        }
        "stop" => {
            agent::stop_agent(config)?;
            println!("Agent stopped");
        }
        "status" => {
            let status = agent::get_agent_status(config)?;
            println!("{}", status);
        }
        _ => {
            return Err(RepomanError::InvalidAgentAction(action.to_string()));
        }
    }
    Ok(())
}
