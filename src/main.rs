use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::provider::ProxyProvider;
use anyhow::Result;
use lazy_static::lazy_static;
use log::{error, info};
use tokio::runtime::Runtime;
use tokio::time::{Instant, interval_at, MissedTickBehavior};
use crate::checker::check_proxy_pool;
use crate::config::Config;
use crate::proxy::Proxy;
use crate::time::current_timestamp;

mod proxy;
mod provider;
mod checker;
mod time;
mod config;

lazy_static! {
    static ref CONFIG: Arc<Mutex<Option<Config>>> = Arc::new(Mutex::new(None));
    static ref PROXY_POOL: Arc<Mutex<BTreeSet<Proxy>>> = Arc::new(Mutex::new(BTreeSet::<Proxy>::new()));
}


fn main() {
    // Init logger
    env_logger::init();
    // Prepare for start up
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
    info!("Starting main thread");
    let main_thread = Runtime::new().unwrap();
    main_thread.spawn(async {
        let proxy_pool_file = Path::new("pool.json");
        if !proxy_pool_file.exists() {
            match save_proxy_pool() {
                Ok(_) => {
                    info!("Successfully create proxy pool file");
                }
                Err(error) => {
                    error!("Create proxy pool file failed, {}", error);
                }
            }
        } else {
            match load_proxy_pool() {
                Ok(proxy_pool) => {
                    {
                        let mut global_proxy_pool = PROXY_POOL.lock().unwrap();
                        *global_proxy_pool = proxy_pool;
                        info!("Current proxy count: {}", global_proxy_pool.len());
                    }
                    info!("Successfully load proxy pool file, now checking...");
                }
                Err(error) => {
                    error!("Load proxy pool file failed, {}", error);
                }
            }
        }
        {
            let mut proxies = provider::checkerproxy::CheckerProxyProvider::new();
            let result = proxies.fetch().await.unwrap();
            let mut proxy_pool = PROXY_POOL.lock().unwrap();
            for proxy in result {
                proxy_pool.insert(proxy);
            }
        }
        save_proxy_pool().unwrap();
    });
    info!("Starting repeat task thread");
    let repeat_task_thread = Runtime::new().unwrap();
    // Block initial thread on background repeat task
    repeat_task_thread.block_on(async {
        let duration = Duration::from_secs(Arc::clone(&CONFIG).lock().unwrap().as_ref().unwrap().check_interval);
        let mut interval = interval_at(
            Instant::now() + duration, duration,
        );
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            info!("Checking proxy pool {}", current_timestamp());
            check_proxy_pool().await.unwrap();
            save_proxy_pool().unwrap();
        }
    });
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

fn save_proxy_pool() -> Result<()> {
    let mut file = File::create("pool.json")?;
    let json = serde_json::to_string(PROXY_POOL.lock().unwrap().deref())?;
    file.write_all(json.as_bytes()).map_err(anyhow::Error::from)
}

fn load_proxy_pool() -> Result<BTreeSet<Proxy>> {
    let mut file = File::open("pool.json")?;
    let mut json: String = String::new();
    file.read_to_string(&mut json)?;
    let proxy_pool: BTreeSet<Proxy> = serde_json::from_str(json.as_str())?;
    Ok(proxy_pool)
}