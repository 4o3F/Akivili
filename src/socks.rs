use std::io::ErrorKind;
use std::net::ToSocketAddrs;
use std::ops::Deref;

use anyhow::{anyhow, Context, Result};
use fast_socks5::client;
use fast_socks5::client::Socks5Stream;
use fast_socks5::server::{Config, Socks5Server, Socks5Socket};
use log::{error, info};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;

use crate::{CONFIG, PROXY_POOL};
use crate::proxy::{Proxy, ProxyType};
use crate::time::current_timestamp;

pub async fn init_socks_server() -> Result<()> {
    info!("Initializing socks server");
    let global_config = CONFIG.lock().unwrap().deref().clone().unwrap();
    let mut server_config: Config = Config::default();
    server_config.set_request_timeout(global_config.socks_server_timeout);
    server_config.set_dns_resolve(false);
    server_config.set_transfer_data(false);
    let listen_addr = format!("127.0.0.1:{}", global_config.socks_server_port);
    let mut server = Socks5Server::bind(&listen_addr).await?;
    server.set_config(server_config);
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
                    if let Err(err) = handle_socket(socket, selected_proxy.clone()).await {
                        error!("Socks server handle error, {:#}", err);
                    }
                });
            }
            Err(err) => {
                error!("Socks server accept error, {:?}", err);
            }
        }
    }
    Ok(())
}

async fn handle_socket<T>(socket: Socks5Socket<T>, proxy: Proxy) -> Result<()>
    where
        T: AsyncRead + AsyncWrite + Unpin,
{
    // upgrade socket to SOCKS5 proxy
    let mut socks5_socket = socket
        .upgrade_to_socks5()
        .await
        .context("Upgrade incoming socket to socks5")?;

    // get resolved target addr
    socks5_socket
        .resolve_dns()
        .await
        .context("Resolve target dns for incoming socket")?;
    let socket_addr = socks5_socket
        .target_addr()
        .context("Find target address for incoming socket")?
        .to_socket_addrs()
        .context("Convert target address of incoming socket to socket addresses")?
        .next()
        .context("Reach out to target of incoming socket")?;

    match proxy.proxy_type {
        ProxyType::SOCKS5 => {
            let mut stream = Socks5Stream::connect(
                format!("{}:{}", proxy.proxy_ip, proxy.proxy_port.to_string()),
                socket_addr.ip().to_string(),
                socket_addr.port(),
                client::Config::default(),
            )
                .await
                .context("Connect to downstream proxy for incoming socket")?;
            match tokio::io::copy_bidirectional(&mut stream, &mut socks5_socket).await {
                Ok(_) => {
                    Ok(())
                }
                Err(err) => match err.kind() {
                    ErrorKind::NotConnected => {
                        Ok(())
                    }
                    ErrorKind::ConnectionReset => {
                        Ok(())
                    }
                    _ => Err(anyhow!(
                            "Socket transfer error, {:#}",
                            err
                        ))
                },
            }
        }
        // ProxyType::HTTP => {
        //
        // }
        _ => {
            Err(anyhow!("Unsupported protocol"))
        }
    }
    // copy data between our incoming client and the used downstream proxy
}