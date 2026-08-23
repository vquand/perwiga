//! Optional HTTP transport for feed refreshes.

use std::time::Duration;

use crate::{
    error::{PerwigaError, Result},
    feed::{normalize_feed_items, parse_feed},
    model::FeedItem,
};

pub trait FeedTransport {
    fn fetch(&self, url: &str) -> Result<String>;
}

pub struct HttpFeedTransport {
    attempts: usize,
    timeout: Duration,
}

const MAX_FEED_BYTES: usize = 8 * 1024 * 1024;

impl HttpFeedTransport {
    pub fn new(attempts: usize, timeout: Duration) -> Result<Self> {
        if attempts == 0 {
            return Err(PerwigaError::Validation(
                "feed transport attempts must be positive".into(),
            ));
        }
        Ok(Self { attempts, timeout })
    }
}

impl Default for HttpFeedTransport {
    fn default() -> Self {
        Self {
            attempts: 3,
            timeout: Duration::from_secs(15),
        }
    }
}

impl FeedTransport for HttpFeedTransport {
    fn fetch(&self, url: &str) -> Result<String> {
        let parsed = url::Url::parse(url)
            .map_err(|error| PerwigaError::Validation(format!("invalid feed URL: {error}")))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(PerwigaError::Validation(
                "feed URL must use http or https and include a host".into(),
            ));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            // Do not silently follow a user-provided source to another scheme or host.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("Perwiga/0.1 local feed reader")
            .build()
            .map_err(|error| PerwigaError::Network(error.to_string()))?;
        let mut last_error = String::from("feed request failed");
        for _ in 0..self.attempts {
            match client.get(url).send() {
                Ok(response) => match response.error_for_status() {
                    Ok(response) => {
                        if response
                            .content_length()
                            .is_some_and(|length| length as usize > MAX_FEED_BYTES)
                        {
                            last_error = format!("feed response exceeds {MAX_FEED_BYTES} bytes");
                            continue;
                        }
                        let bytes = response
                            .bytes()
                            .map_err(|error| PerwigaError::Network(error.to_string()))?;
                        if bytes.len() > MAX_FEED_BYTES {
                            last_error = format!("feed response exceeds {MAX_FEED_BYTES} bytes");
                            continue;
                        }
                        return String::from_utf8(bytes.to_vec())
                            .map_err(|error| PerwigaError::Network(error.to_string()));
                    }
                    Err(error) => last_error = error.to_string(),
                },
                Err(error) => last_error = error.to_string(),
            }
        }
        Err(PerwigaError::Network(last_error))
    }
}

pub fn fetch_and_normalize<T: FeedTransport>(
    transport: &T,
    url: &str,
    discovered_at: &str,
    provenance: &str,
) -> Result<Vec<FeedItem>> {
    let xml = transport.fetch(url)?;
    let parsed = parse_feed(&xml)?;
    normalize_feed_items(&parsed, discovered_at, provenance)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureTransport(&'static str);

    impl FeedTransport for FixtureTransport {
        fn fetch(&self, _url: &str) -> Result<String> {
            Ok(self.0.to_string())
        }
    }

    #[test]
    fn transport_boundary_normalizes_fixture_without_live_network() {
        let transport = FixtureTransport(
            "<rss><channel><item><guid>fixture-1</guid><title>Fixture</title></item></channel></rss>",
        );
        let items = fetch_and_normalize(
            &transport,
            "https://example.test/feed.xml",
            "2026-08-22T00:00:00Z",
            "fixture",
        )
        .expect("fixture transport");
        assert_eq!(items[0].external_identity, "fixture-1");
    }

    #[test]
    fn rejects_non_http_feed_urls_before_transport() {
        let error = HttpFeedTransport::default()
            .fetch("file:///tmp/feed.xml")
            .expect_err("unsafe URL");
        assert!(error.to_string().contains("http"));
    }
}
