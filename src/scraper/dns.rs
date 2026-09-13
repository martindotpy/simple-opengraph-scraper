use std::{collections::HashMap, net::IpAddr};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use crate::scraper::url::{is_blocked_ip, resolve_socket_addresses};

/// DNS resolver serving pre validated IPs: closes the resolve then connect TOCTOU gap.
#[derive(Clone, Default)]
pub struct PinnedDns {
    pinned: std::sync::Arc<tokio::sync::Mutex<HashMap<String, Vec<IpAddr>>>>,
}

impl PinnedDns {
    pub async fn pin(&self, host: &str, addresses: Vec<IpAddr>) {
        self.pinned.lock().await.insert(host.to_string(), addresses);
    }

    pub async fn unpin(&self, host: &str) {
        self.pinned.lock().await.remove(host);
    }
}

impl Resolve for PinnedDns {
    fn resolve(&self, name: Name) -> Resolving {
        let pinned = self.pinned.clone();

        Box::pin(async move {
            if let Some(addresses) = pinned.lock().await.get(name.as_str()) {
                // Port comes from the request URI, not from here
                let resolved_addresses: Addrs = Box::new(
                    addresses
                        .iter()
                        .map(|ip| std::net::SocketAddr::new(*ip, 0))
                        .collect::<Vec<_>>()
                        .into_iter(),
                );

                return Ok(resolved_addresses);
            }

            // Defense in depth for unexpected paths: fail closed like validation does
            let mut allowed = Vec::new();

            for socket_address in resolve_socket_addresses(name.as_str())
                .await
                .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?
            {
                if is_blocked_ip(&socket_address.ip()) {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "host resolves to blocked addresses",
                    ))
                        as Box<dyn std::error::Error + Send + Sync>);
                }

                allowed.push(socket_address);
            }

            let resolved_addresses: Addrs = Box::new(allowed.into_iter());

            Ok(resolved_addresses)
        })
    }
}
