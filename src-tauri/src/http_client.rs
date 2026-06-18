use reqwest::{Client, Proxy};
use std::time::Duration;

pub fn build_clients(timeout: Duration) -> Vec<Client> {
    let mut clients = Vec::new();

    if let Ok(proxy_url) = std::env::var("WORLD_CUP_ISSUE_PROXY")
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
    {
        if let Ok(proxy) = Proxy::all(proxy_url) {
            if let Ok(client) = Client::builder().timeout(timeout).proxy(proxy).build() {
                clients.push(client);
            }
        }
    } else if local_proxy_hint_enabled() {
        if let Ok(proxy) = Proxy::all("http://127.0.0.1:1080") {
            if let Ok(client) = Client::builder().timeout(timeout).proxy(proxy).build() {
                clients.push(client);
            }
        }
    }

    if let Ok(client) = Client::builder().timeout(timeout).build() {
        clients.push(client);
    }

    clients
}

fn local_proxy_hint_enabled() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:1080".parse().expect("static proxy address"),
        Duration::from_millis(200),
    )
    .is_ok()
}
