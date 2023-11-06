use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use log::{error, info};

use crate::{CONFIG, PROXY_POOL};
use crate::proxy::Proxy;
use crate::time::current_timestamp;

pub mod docip;
pub mod checkerproxy;

#[async_trait]
pub trait ProxyProvider {
    const PROXY_IDENTIFIER: &'static str;
    fn new() -> Self;
    fn get_last_fetch(&self) -> u64;
    async fn fetch(&mut self) -> Result<Vec<Proxy>>;
}

pub async fn update_proxy_pool() -> Result<()> {
    let proxies = Arc::new(Mutex::new(Vec::<Proxy>::new()));
    let config = (*CONFIG.lock().unwrap()).clone().unwrap();
    let mut tasks = Vec::new();
    //let join_set = JoinSet::new();
    if config.provider_checkerproxy_enabled {
        let proxies = Arc::clone(&proxies);
        tasks.push(tokio::spawn(async move {
            info!("Updating CheckerProxy provider");
            let mut provider = checkerproxy::CheckerProxyProvider::new();
            match provider.fetch().await {
                Ok(mut new_proxies) => {
                    proxies.lock().unwrap().append(new_proxies.as_mut());
                    info!("CheckerProxy provider update finished")
                }
                Err(error) => {
                    error!("Update CheckProxy provider failed {}", error)
                }
            };
        }));
    }
    if config.provider_docip_enabled {
        let proxies = Arc::clone(&proxies);
        tasks.push(tokio::spawn(async move {
            info!("Updating DocIP provider");
            let mut provider = docip::DocIPProvider::new();
            match provider.fetch().await {
                Ok(mut new_proxies) => {
                    proxies.lock().unwrap().append(new_proxies.as_mut());
                    info!("DocIP provider update finished")
                }
                Err(error) => {
                    error!("Update DocIP provider failed {}", error)
                }
            };
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }
    {
        let mut proxy_pool = PROXY_POOL.lock().unwrap();
        for proxy in proxies.lock().unwrap().iter() {
            proxy_pool.insert((*proxy).clone());
        }
    }
    info!("Update proxy pool successfully at {}", current_timestamp());
    Ok(())
}