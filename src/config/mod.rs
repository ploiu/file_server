use config::{Config, ConfigError};
use once_cell::sync::Lazy;
use rocket::form::validate::Contains;
use rocket::serde::Deserialize;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::panic::panic_any;

/// config properties for the rabbit queue
#[derive(Deserialize, Clone)]
pub struct RabbitMqConfig {
    pub address: String,
}

/// config properties for the whole of this application
#[derive(Deserialize, Clone)]
pub struct FileServerConfig {
    #[serde(rename = "rabbitmq")]
    pub rabbit_mq: RabbitMqConfig,
}

/// Parses the config file located at ./FileServer.toml, if it exists.
/// If this fails to parse the file, the application will panic
pub fn parse_config() -> Option<FileServerConfig> {
    let builder = Config::builder()
        .add_source(config::File::with_name("./FileServers.toml"))
        .build();
    // some errors are fine, such as not found
    if let Err(ConfigError::Foreign(e)) = builder {
        let message = e.to_string();
        if message.contains("not found") {
            log::warn!("No config file found. Continuing startup...");
            return None;
        }
        panic!("Failed to parse config file. Exception is {e}");
        // basically everything else is unrecoverable, though
    } else if let Err(e) = builder {
        log::error!("Failed to parse config file. Exception is {e}");
        panic!("Failed to parse config file. Exception is {e}");
    }
    let settings = builder.unwrap();
    // let x = settings.try_deserialize::<HashMap<String, HashMap<String, String>>>().unwrap();
    let config: FileServerConfig = settings.try_deserialize().unwrap();
    Some(config)
}

/// global variable for config, that way it doesn't need to be repeatedly parsed
pub static FILE_SERVER_CONFIG: Lazy<Option<FileServerConfig>> = Lazy::new(parse_config);
