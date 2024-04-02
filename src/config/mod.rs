use std::string::ToString;

use config::{Config, ConfigError};
use once_cell::sync::Lazy;
use rocket::form::validate::Contains;
use rocket::serde::Deserialize;

/// config properties for the rabbit queue
#[derive(Deserialize, Clone)]
pub struct RabbitMqConfig {
    pub address: Option<String>,
    pub enabled: bool,
}

#[derive(Deserialize, Clone)]
pub struct DbConfig {
    pub location: String,
}

#[derive(Deserialize, Clone)]
pub struct FilePreviewConfig {
    #[serde(rename = "sleeptimemillis")]
    pub sleep_time_millis: u32,
    #[serde(rename = "itemstoprocessperbatch")]
    pub items_to_process_per_batch: u32,
}

/// config properties for the whole of this application
#[derive(Deserialize, Clone)]
pub struct FileServerConfig {
    #[serde(rename = "rabbitmq")]
    pub rabbit_mq: RabbitMqConfig,
    #[serde(rename = "filepreview")]
    pub file_preview: FilePreviewConfig,
    pub database: DbConfig,
}

/// Parses the config file located at ./FileServer.toml, if it exists.
/// If this fails to parse the file, the application will panic
pub fn parse_config() -> FileServerConfig {
    let builder = Config::builder()
        .add_source(config::File::with_name("./FileServer.toml"))
        .build();
    // some errors are fine, such as not found
    if let Err(ConfigError::Foreign(e)) = builder {
        let message = e.to_string();
        if message.contains("not found") {
            log::warn!("No config file found. Continuing startup...");
            return FS_CONFIG_DEFAULT.clone();
        }
        panic!("Failed to parse config file. Exception is {e}");
        // basically everything else is unrecoverable, though
    } else if let Err(e) = builder {
        log::error!("Failed to parse config file. Exception is {e}");
        panic!("Failed to parse config file. Exception is {e}");
    }
    let settings = builder.unwrap();
    settings
        .try_deserialize()
        .unwrap_or(FS_CONFIG_DEFAULT.clone())
}

/// global variable for config, that way it doesn't need to be repeatedly parsed
pub static FILE_SERVER_CONFIG: Lazy<FileServerConfig> = Lazy::new(parse_config);
static FS_CONFIG_DEFAULT: Lazy<FileServerConfig> = Lazy::new(|| FileServerConfig {
    rabbit_mq: RabbitMqConfig {
        address: Some("amqp://127.0.0.1:5672".to_string()),
        enabled: true,
    },
    file_preview: FilePreviewConfig {
        sleep_time_millis: 300_000,
        items_to_process_per_batch: 20,
    },
    database: DbConfig {
        location: "./db.sqlite".to_string(),
    },
});
