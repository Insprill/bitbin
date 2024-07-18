use anyhow::{bail, Result};
use std::{fs, path::PathBuf};

use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub bitbin: BitbinConfig,
}

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    /// Sets the address to listen on
    pub host: String,

    /// Sets the port to listen on. Will be overriden by the PORT env var if present
    pub port: u16,

    /// The amount of HTTP workers to use. 0 to equal physical CPU cores
    pub workers: usize,

    /// The Keep-Alive timeout, in seconds. Set to 0 to disable.
    pub keep_alive_timeout: f32,

    /// Whether TLS should be used
    pub tls: bool,

    /// The path to the KEY file. Required when using TLS.
    pub tls_key_file: Option<String>,

    /// The path to the CERT file. Required when using TLS.
    pub tls_cert_file: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
pub struct BitbinConfig {
    /// Max content length in MB
    pub max_content_length: usize,
    /// The length of generated keys in characters
    pub key_length: usize,
}

impl Config {
    pub fn create() -> Result<Config> {
        if !PathBuf::from("config.toml").exists() {
            return Ok(Config::default());
        }

        let raw_config = fs::read_to_string("config.toml")?;

        let config: Config = match toml::from_str(&raw_config) {
            Ok(cfg) => cfg,
            Err(err) => {
                bail!("Failed to read config file! {}", err);
            }
        };

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 0,
                keep_alive_timeout: 15.0,
                tls: false,
                tls_key_file: Option::None,
                tls_cert_file: Option::None,
            },
            bitbin: BitbinConfig {
                max_content_length: 10,
                key_length: 6,
            },
        }
    }
}
