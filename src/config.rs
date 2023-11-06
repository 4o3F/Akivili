use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::CONFIG;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub check_timeout: u64,
    pub check_interval: u64,
    pub update_interval: u64,
    pub socks_server_port: u64,
    pub socks_server_timeout: u64,
    pub provider_docip_enabled: bool,
    pub provider_checkerproxy_enabled: bool,
}

impl Config {
    pub fn default() -> Config {
        Config {
            check_timeout: 10,
            check_interval: 300,
            update_interval: 6000,
            socks_server_port: 2333,
            socks_server_timeout: 10,
            provider_docip_enabled: true,
            provider_checkerproxy_enabled: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::default()
    }
}

pub fn init_config() {
    info!("Initializing config");
    let config_file = Path::new("config.yaml");
    if !config_file.exists() {
        match create_config() {
            Ok(_) => {
                let mut global_config = CONFIG.lock().unwrap();
                *global_config = Some(Config::default());
                info!("Successfully create config file");
            }
            Err(error) => {
                error!("Create config file failed, {}", error);
            }
        }
    } else {
        match load_config() {
            Ok(config) => {
                let mut global_config = CONFIG.lock().unwrap();
                *global_config = Some(config);
                info!("Successfully load config file");
            }
            Err(error) => {
                error!("Load config file failed, {}", error);
            }
        }
    }
}

fn create_config() -> Result<()> {
    let mut file = File::create("config.yaml")?;
    let config = Config::default();
    let yaml = serde_yaml::to_string(&config)?;
    file.write_all(yaml.as_bytes()).map_err(anyhow::Error::from)
}

fn load_config() -> Result<Config> {
    let mut file = File::open("config.yaml")?;
    let mut yaml: String = String::new();
    file.read_to_string(&mut yaml)?;
    let config: Config = serde_yaml::from_str(yaml.as_str())?;
    Ok(config)
}