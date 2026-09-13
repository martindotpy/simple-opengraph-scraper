use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::Url;

// Error
#[derive(Debug)]
pub enum DeniedUrlError {
    InvalidUrl,
    NotHttp,
    Credentials,
    ForbiddenPort,
    ForbiddenHost,
    Unresolvable,
}

/// Full check: syntax plus DNS. Returns the URL and its validated IPs for DNS pinning.
pub async fn validate_public_http_url(raw_url: &str) -> Result<(Url, Vec<IpAddr>), DeniedUrlError> {
    let url = Url::parse(raw_url).map_err(|_| DeniedUrlError::InvalidUrl)?;

    validate_http_url(&url)?;

    let host = url.host_str().unwrap_or_default();

    // Literal already screened by validate_host: no DNS needed
    if let Ok(literal) = parse_ip_literal(host) {
        return Ok((url, vec![literal]));
    }

    let addresses = resolve_public_ips(host).await?;

    Ok((url, addresses))
}

pub fn validate_http_url(url: &Url) -> Result<(), DeniedUrlError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(DeniedUrlError::NotHttp);
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(DeniedUrlError::Credentials);
    }

    if let Some(port) = url.port()
        && port != 80
        && port != 443
    {
        return Err(DeniedUrlError::ForbiddenPort);
    }

    validate_host(url.host_str().unwrap_or_default())?;

    Ok(())
}

/// Single DNS lookup shared by validation and the pinned resolver.
pub(crate) async fn resolve_socket_addresses(
    host: &str,
) -> Result<Vec<std::net::SocketAddr>, std::io::Error> {
    let addresses: Vec<std::net::SocketAddr> =
        tokio::net::lookup_host((host, 443)).await?.collect();

    if addresses.is_empty() {
        return Err(std::io::Error::other("host resolved to no addresses"));
    }

    Ok(addresses)
}

/// Resolves and keeps only public IPs: any blocked hit fails the whole host closed.
pub async fn resolve_public_ips(host: &str) -> Result<Vec<IpAddr>, DeniedUrlError> {
    let mut addresses = Vec::new();

    for socket_address in resolve_socket_addresses(host)
        .await
        .map_err(|_| DeniedUrlError::Unresolvable)?
    {
        if is_blocked_ip(&socket_address.ip()) {
            return Err(DeniedUrlError::ForbiddenHost);
        }

        addresses.push(socket_address.ip());
    }

    Ok(addresses)
}

/// Validates the host without allocating.
fn validate_host(host: &str) -> Result<(), DeniedUrlError> {
    // Hosts from Url::parse are already lowercased (WHATWG URL, special schemes)
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") {
        return Err(DeniedUrlError::ForbiddenHost);
    }

    // Prevent access to private IP ranges and other reserved addresses
    if let Ok(address) = parse_ip_literal(host)
        && is_blocked_ip(&address)
    {
        return Err(DeniedUrlError::ForbiddenHost);
    }

    Ok(())
}

pub(crate) fn parse_ip_literal(host: &str) -> Result<IpAddr, ()> {
    // Normalization
    let trimmed = host.trim_matches(['[', ']']);

    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        return Ok(ip);
    }

    if !trimmed.contains('.') {
        // Single 32-bit number: "2130706433" == 127.0.0.1
        if trimmed.is_empty() {
            return Err(());
        }

        let number = parse_int_u32(trimmed).map_err(|_| ())?;

        return Ok(IpAddr::V4(number.into()));
    }

    // Four octets: "127.0.0.1" (decimal, octal or hex per octet)
    let mut octets = [0u8; 4];
    let mut count = 0;

    for part in trimmed.split('.') {
        if count == 4 {
            return Err(());
        }

        octets[count] = parse_int_octet(part).map_err(|_| ())?;
        count += 1;
    }

    if count == 4 {
        return Ok(IpAddr::V4(octets.into()));
    }

    Err(())
}

fn parse_int_octet(text: &str) -> Result<u8, ()> {
    let number = parse_int_u32(text).map_err(|_| ())?;

    u8::try_from(number).map_err(|_| ())
}

fn parse_int_u32(text: &str) -> Result<u32, ()> {
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).map_err(|_| ())
    } else if text.len() > 1
        && text.starts_with('0')
        && text.chars().all(|character| character.is_ascii_digit())
    {
        u32::from_str_radix(text, 8).map_err(|_| ())
    } else {
        text.parse::<u32>().map_err(|_| ())
    }
}

pub(crate) fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_private()
                || ipv4.is_loopback()
                || ipv4.is_link_local()
                || ipv4.is_multicast()
                || ipv4.is_unspecified()
                || ipv4.is_broadcast()
                || ipv4.is_documentation()
                || in_v4_cidr(ipv4, [100, 64, 0, 0], 10) // shared CGNAT
                || in_v4_cidr(ipv4, [192, 0, 0, 0], 24) // IETF assignments
                || in_v4_cidr(ipv4, [198, 18, 0, 0], 15) // benchmarking
                || in_v4_cidr(ipv4, [0, 0, 0, 0], 8) // software scope
                || in_v4_cidr(ipv4, [240, 0, 0, 0], 4) // reserved for future use
        }
        IpAddr::V6(ipv6) => {
            if let Some(mapped) = ipv6.to_ipv4_mapped() {
                return is_blocked_ip(&IpAddr::V4(mapped)); // ::ffff:127.0.0.1
            }

            ipv6.is_loopback()
                || ipv6.is_multicast()
                || ipv6.is_unspecified()
                || ipv6.is_unique_local() // fc00::/7
                || ipv6.is_unicast_link_local() // fe80::/10
                || is_documentation_v6(ipv6) // 2001:db8::/32
                || is_nat64(ipv6) // 64:ff9b::/96
        }
    }
}

fn in_v4_cidr(ip: &Ipv4Addr, network: [u8; 4], prefix: u32) -> bool {
    let mask = u32::MAX << (32 - prefix);

    (u32::from(*ip) & mask) == (u32::from(Ipv4Addr::from(network)) & mask)
}

fn is_documentation_v6(ip: &Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
}

fn is_nat64(ip: &Ipv6Addr) -> bool {
    ip.segments()[..6] == [0x64, 0xff9b, 0, 0, 0, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_private_literals() {
        for host in [
            "127.0.0.1",
            "0x7f.0.0.1",
            "2130706433",
            "10.0.0.1",
            "192.168.1.1",
            "0.0.0.0",
            "0.1.2.3",
            "100.64.0.1",
            "198.18.0.1",
            "192.0.0.1",
            "169.254.169.254",
            "240.0.0.1",
            "LOCALHOST",
            "evil.localhost",
            "[::1]",
            "[::ffff:127.0.0.1]",
            "[fc00::1]",
            "[fe80::1]",
        ] {
            let url = Url::parse(&format!("http://{host}/")).unwrap();

            assert!(validate_http_url(&url).is_err(), "{host}");
        }
    }

    #[test]
    fn allows_public_literals() {
        for host in ["93.184.215.14", "[2606:4700:4700::1111]"] {
            let url = Url::parse(&format!("http://{host}/")).unwrap();

            assert!(validate_http_url(&url).is_ok(), "{host}");
        }
    }

    #[test]
    fn blocks_ports_and_creds() {
        let url = Url::parse("http://example.com:22/").unwrap();

        assert!(matches!(
            validate_http_url(&url),
            Err(DeniedUrlError::ForbiddenPort)
        ));

        let url = Url::parse("http://user@example.com/").unwrap();

        assert!(matches!(
            validate_http_url(&url),
            Err(DeniedUrlError::Credentials)
        ));

        let url = Url::parse("ftp://example.com/").unwrap();

        assert!(matches!(
            validate_http_url(&url),
            Err(DeniedUrlError::NotHttp)
        ));
    }

    #[tokio::test]
    async fn blocks_localhost_dns() {
        assert!(matches!(
            resolve_public_ips("localhost").await,
            Err(DeniedUrlError::ForbiddenHost)
        ));
    }
}
