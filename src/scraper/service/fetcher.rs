use crate::{
    core::{dns::PinnedDns, url_guard},
    scraper::domain::ScrapeError,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::warn;
use url::Url;

const MAX_REDIRECTS: u8 = 3;

/// Meta crawlers: generic bot UAs get 403 from some large sites (e.g. MercadoLibre).
/// On server errors the retry goes out with the next UA in rotation.
const USER_AGENTS: [&str; 3] = [
    "WhatsApp/2.22.20.72 A",
    "facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)",
    "Meta-ExternalAgent/1.1 (+https://developers.facebook.com/docs/sharing/webmasters/crawler)",
];

static NEXT_USER_AGENT: AtomicUsize = AtomicUsize::new(0);

fn user_agent_for_attempt(start: usize, attempt: usize) -> &'static str {
    USER_AGENTS[(start + attempt) % USER_AGENTS.len()]
}

/// Fetches with per-hop SSRF validation: every redirect target is checked and
/// DNS pinned before requesting, so only validated IPs are ever connected to.
pub(crate) async fn fetch_validated(
    http_client: &reqwest::Client,
    dns: &PinnedDns,
    raw_url: &str,
    host: &str,
) -> Result<reqwest::Response, ScrapeError> {
    let mut current_url = raw_url.to_string();
    let mut pinned_hosts: Vec<String> = Vec::new();

    let result = fetch_loop(http_client, dns, &mut current_url, host, &mut pinned_hosts).await;

    for pinned_host in pinned_hosts {
        dns.unpin(&pinned_host).await;
    }

    result
}

async fn fetch_loop(
    http_client: &reqwest::Client,
    dns: &PinnedDns,
    current_url: &mut String,
    host: &str,
    pinned_hosts: &mut Vec<String>,
) -> Result<reqwest::Response, ScrapeError> {
    for _ in 0..=MAX_REDIRECTS {
        let (checked_url, addresses) = url_guard::validate_public_http_url(current_url)
            .await
            .map_err(|denied| {
                warn!(reason = ?denied, "rejected unsafe url");

                denied
            })?;

        let Some(hop_host) = checked_url.host_str() else {
            return Err(ScrapeError::Unexpected);
        };

        // Literals need no DNS: skip the pin map churn
        if url_guard::parse_ip_literal(hop_host).is_err() {
            dns.pin(hop_host, addresses).await;
            pinned_hosts.push(hop_host.to_string());
        }

        let response = fetch_with_soft_retry(http_client, checked_url.as_str(), hop_host).await?;

        let Some(next_url) = redirect_target(&response) else {
            return Ok(response);
        };

        *current_url = next_url;
    }

    warn!(%host, "too many redirects");

    Err(ScrapeError::Upstream)
}

/// Next hop URL for redirects, resolved against the current one.
fn redirect_target(response: &reqwest::Response) -> Option<String> {
    resolve_redirect_target(
        response.url(),
        response.status(),
        response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|header| header.to_str().ok()),
    )
}

fn resolve_redirect_target(
    base_url: &Url,
    status: reqwest::StatusCode,
    location: Option<&str>,
) -> Option<String> {
    if !status.is_redirection() {
        return None;
    }

    let target = base_url.join(location?).ok()?;

    if !matches!(target.scheme(), "http" | "https") {
        return None;
    }

    Some(target.to_string())
}

/// One soft retry: repeat once on connect failure or 502/503.
async fn fetch_with_soft_retry(
    http_client: &reqwest::Client,
    url: &str,
    host: &str,
) -> Result<reqwest::Response, ScrapeError> {
    let user_agent_start = NEXT_USER_AGENT.fetch_add(1, Ordering::Relaxed);

    for attempt in 0..2 {
        match http_client
            .get(url)
            .header(reqwest::header::ACCEPT, "text/html")
            .header(
                reqwest::header::USER_AGENT,
                user_agent_for_attempt(user_agent_start, attempt),
            )
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                if (status == reqwest::StatusCode::BAD_GATEWAY
                    || status == reqwest::StatusCode::SERVICE_UNAVAILABLE)
                    && attempt == 0
                {
                    continue;
                }

                return Ok(response);
            }
            Err(error) => {
                if error.is_connect() && attempt == 0 {
                    continue;
                }
                warn!(%host, error = ?error, "upstream fetch failed");

                return Err(map_fetch_error(error));
            }
        }
    }

    unreachable!("retry loop must return")
}

pub(crate) fn map_fetch_error(error: reqwest::Error) -> ScrapeError {
    if error.is_timeout() {
        return ScrapeError::Timeout;
    }

    ScrapeError::Upstream
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_url() -> Url {
        Url::parse("https://example.com/a/b").unwrap()
    }

    #[test]
    fn follows_http_redirects() {
        for status in [301, 302, 303, 307, 308] {
            let target = resolve_redirect_target(
                &base_url(),
                reqwest::StatusCode::from_u16(status).unwrap(),
                Some("/c"),
            );

            assert_eq!(target.as_deref(), Some("https://example.com/c"));
        }
    }

    #[test]
    fn resolves_relative_locations() {
        let target = resolve_redirect_target(&base_url(), reqwest::StatusCode::FOUND, Some("c"));

        assert_eq!(target.as_deref(), Some("https://example.com/a/c"));
    }
    #[test]
    fn ignores_non_redirects_and_bad_targets() {
        assert_eq!(
            resolve_redirect_target(&base_url(), reqwest::StatusCode::OK, Some("/c")),
            None
        );
        assert_eq!(
            resolve_redirect_target(&base_url(), reqwest::StatusCode::FOUND, None),
            None
        );
        assert_eq!(
            resolve_redirect_target(
                &base_url(),
                reqwest::StatusCode::FOUND,
                Some("javascript:alert(1)")
            ),
            None
        );
        assert_eq!(
            resolve_redirect_target(&base_url(), reqwest::StatusCode::FOUND, Some("ftp://x/y")),
            None
        );
    }

    #[test]
    fn rotates_user_agents_on_retry() {
        assert_eq!(user_agent_for_attempt(0, 0), USER_AGENTS[0]);
        assert_eq!(user_agent_for_attempt(0, 1), USER_AGENTS[1]);
        assert_eq!(user_agent_for_attempt(1, 0), USER_AGENTS[1]);
        assert_eq!(user_agent_for_attempt(1, 1), USER_AGENTS[2]);
        assert_eq!(user_agent_for_attempt(2, 1), USER_AGENTS[0]);
    }
}
