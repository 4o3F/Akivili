use std::collections::BTreeSet;
use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lazy_static::lazy_static;
use log::info;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::{Instant, interval_at, MissedTickBehavior};

use crate::checker::check_proxy_pool;
use crate::config::Config;
use crate::provider::update_proxy_pool;
use crate::proxy::{Proxy, save_proxy_pool};
use crate::socks::init_socks_server;
use crate::time::current_timestamp;

mod proxy;
mod provider;
mod checker;
mod time;
mod config;
mod socks;

lazy_static! {
    static ref CONFIG: Arc<Mutex<Option<Config>>> = Arc::new(Mutex::new(None));
    static ref PROXY_POOL: Arc<Mutex<BTreeSet<Proxy>>> = Arc::new(Mutex::new(BTreeSet::<Proxy>::new()));
}

async fn test() {
    let mut stream = TcpStream::connect("222.220.102.159:8000").await.unwrap();
    let connect_head = String::from("CONNECT myip.ipip.net:443 HTTP/1.1\r\nHost: myip.ipip.net:443\r\nUser-Agent: curl/8.0.1\r\nProxy-Connection: Keep-Alive\r\n\r\n");
    stream.write_all(connect_head.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    info!("{}",response);
}

#[tokio::main]
async fn main() {
    // Init logger
    env_logger::init();
    // Prepare for start up
    config::init_config();
    proxy::init_proxy_pool();
    // Preparation finished
    info!("Starting main thread");
    let main_thread = Runtime::new().unwrap();
    main_thread.spawn(async {
        // TODO: implement socks5 server main thread
    });
    info!("Starting proxy pool check timer");
    let proxy_pool_check_task = Runtime::new().unwrap();
    // Block initial thread on background repeat task
    proxy_pool_check_task.spawn(async {
        let duration = Duration::from_secs(Arc::clone(&CONFIG).lock().unwrap().as_ref().unwrap().check_interval);
        let mut interval = interval_at(
            Instant::now(), duration,
        );
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            info!("Checking proxy pool {}", current_timestamp());
            check_proxy_pool().await.unwrap();
            save_proxy_pool().unwrap();
        }
    });

    // Block the program on this thread
    info!("Starting proxy pool updater timer");
    let proxy_pool_update_task = Runtime::new().unwrap();
    proxy_pool_update_task.spawn(async {
        let duration = Duration::from_secs(Arc::clone(&CONFIG).lock().unwrap().as_ref().unwrap().update_interval);
        let mut interval = interval_at(
            Instant::now(), duration,
        );
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            info!("Updating proxy pool {}", current_timestamp());
            update_proxy_pool().await.unwrap();
            save_proxy_pool().unwrap();
        }
    });

    init_socks_server().await.unwrap();
}

