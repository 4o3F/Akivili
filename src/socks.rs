use std::ops::Deref;

use anyhow::Result;
use fast_socks5::server::{Config, Socks5Server};
use log::{error, info};
use tokio_stream::StreamExt;

use crate::{CONFIG, PROXY_POOL};
use crate::proxy::Proxy;
use crate::time::current_timestamp;

pub async fn init_socks_server() -> Result<()> {
    info!("Initializing socks server");
    let global_config = CONFIG.lock().unwrap().deref().clone().unwrap();
    let mut server_config: Config = Config::default();
    server_config.set_request_timeout(global_config.socks_server_timeout);
    server_config.set_allow_no_auth(true);
    server_config.set_dns_resolve(false);
    let listen_addr = format!("127.0.0.1:{}", global_config.socks_server_port);
    let mut server = <Socks5Server>::bind(&listen_addr).await?;
    server = server.with_config(server_config);
    let mut incoming = server.incoming();
    info!("Socks server listening at {}", listen_addr);
    while let Some(socket_res) = incoming.next().await {
        match socket_res {
            Ok(socket) => {
                let selected_proxy: Proxy;
                {
                    let mut proxy_pool = PROXY_POOL.lock().unwrap();
                    let mut proxy = proxy_pool.pop_first().unwrap();
                    proxy.last_used = current_timestamp();
                    proxy_pool.insert(proxy.clone());
                    selected_proxy = proxy;
                }

                tokio::spawn(async move {
                    // TODO: implement proxy chain
                });
            }
            Err(err) => {
                error!("Socks server accept error, {:?}", err);
            }
        }
    }
    Ok(())
}