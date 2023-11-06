use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use log::info;
use tokio::sync::Semaphore;

use crate::{CONFIG, PROXY_POOL};
use crate::proxy::{Proxy, ProxyType};
use crate::time::current_timestamp;

pub async fn check_proxy_pool() -> Result<()> {
    let proxy_pool: BTreeSet<Proxy>;
    proxy_pool = Arc::clone(&PROXY_POOL).lock().unwrap().iter().map(|proxy| (*proxy).clone()).collect();
    let mut tasks = Vec::new();
    let semaphore = Arc::new(Semaphore::new(10));
    for proxy in proxy_pool.iter() {
        let mut proxy = (*proxy).clone();
        let proxy_pool = Arc::clone(&PROXY_POOL);
        let semaphore = Arc::clone(&semaphore);
        tasks.push(tokio::spawn(async move {
            let _ = semaphore.acquire().await.unwrap();
            let ip = check_proxy(&proxy).await;
            match ip {
                Ok(_) => {
                    proxy.last_checked = current_timestamp();
                    proxy_pool.lock().unwrap().insert(proxy.clone());
                    info!("Updated proxy {}:{}", proxy.proxy_ip, proxy.proxy_port);
                }
                Err(_) => {
                    proxy_pool.lock().unwrap().remove(&proxy);
                    info!("Removed proxy {}:{}", proxy.proxy_ip, proxy.proxy_port);
                }
            }
        }));
    }
    for task in tasks {
        task.await?;
    }
    Ok(())
}

pub async fn check_proxy(proxy: &Proxy) -> Result<String> {
    let mut proxy_scheme = String::new();
    match proxy.proxy_type {
        ProxyType::HTTP => {
            proxy_scheme += "http://";
        }
        ProxyType::HTTPS => {
            proxy_scheme += "https://"
        }
        ProxyType::SOCKS5 => {
            proxy_scheme += "socks5://"
        }
        ProxyType::SOCKS4 => {
            proxy_scheme += "socks4://"
        }
    }
    proxy_scheme += proxy.proxy_ip.as_str();
    proxy_scheme += ":";
    proxy_scheme += proxy.proxy_port.to_string().as_str();
    let client = reqwest::ClientBuilder::new().proxy(
        reqwest::Proxy::all(proxy_scheme.as_str())?
    ).connect_timeout(Duration::from_secs(CONFIG.lock().unwrap().as_ref().unwrap().check_timeout))
        .build()?;
    // https://api.ip.sb/ip
    // https://myip.ipip.net/s
    let request = client.get("https://myip.ipip.net/s").build()?;
    let response = client.execute(request).await?;
    let ip = response.text().await?;
    Ok(ip)
}