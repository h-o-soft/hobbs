//! RSS feed fetcher with security measures.
//!
//! This module provides functionality to fetch and parse RSS/Atom feeds
//! with SSRF protection and resource limits.

use crate::error::{HobbsError, Result};
use crate::rss::types::{ParsedFeed, ParsedItem, MAX_DESCRIPTION_LENGTH, MAX_FEED_SIZE};
use feed_rs::parser;
use reqwest::Client;
use std::net::IpAddr;
use std::time::Duration;

/// Connect timeout in seconds.
const CONNECT_TIMEOUT_SECS: u64 = 10;

/// Read timeout in seconds.
const READ_TIMEOUT_SECS: u64 = 20;

/// Total timeout in seconds.
const TOTAL_TIMEOUT_SECS: u64 = 30;

/// Maximum number of redirects to follow.
const MAX_REDIRECTS: usize = 5;

/// User agent string for feed fetching.
const USER_AGENT: &str = "HOBBS-BBS/1.0 (RSS Reader)";

/// RSS feed fetcher with security measures.
pub struct RssFetcher {
    client: Client,
}

impl RssFetcher {
    /// Create a new fetcher with default settings.
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .read_timeout(Duration::from_secs(READ_TIMEOUT_SECS))
            .timeout(Duration::from_secs(TOTAL_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| HobbsError::Rss(format!("failed to create HTTP client: {}", e)))?;

        Ok(Self { client })
    }

    /// Fetch and parse a feed from the given URL.
    ///
    /// This method performs SSRF validation and enforces size limits.
    pub async fn fetch(&self, url: &str) -> Result<ParsedFeed> {
        // Validate URL for SSRF
        validate_url(url)?;

        // Fetch the feed
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| HobbsError::Rss(format!("failed to fetch feed: {}", e)))?;

        // Check response status
        if !response.status().is_success() {
            return Err(HobbsError::Rss(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // Check content length if available
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_FEED_SIZE {
                return Err(HobbsError::Rss(format!(
                    "feed too large: {} bytes (max {} bytes)",
                    content_length, MAX_FEED_SIZE
                )));
            }
        }

        // Read body
        let bytes = response
            .bytes()
            .await
            .map_err(|e| HobbsError::Rss(format!("failed to read response: {}", e)))?;

        // Check actual size
        if bytes.len() as u64 > MAX_FEED_SIZE {
            return Err(HobbsError::Rss(format!(
                "feed too large: {} bytes (max {} bytes)",
                bytes.len(),
                MAX_FEED_SIZE
            )));
        }

        // Parse the feed
        parse_feed(&bytes)
    }
}

impl Default for RssFetcher {
    fn default() -> Self {
        Self::new().expect("failed to create default RssFetcher")
    }
}

/// Fetch and parse a feed from the given URL (standalone function).
///
/// This is a convenience function that creates a temporary fetcher.
pub async fn fetch_feed(url: &str) -> Result<ParsedFeed> {
    // Validate URL for SSRF
    validate_url(url)?;

    // Create client
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .read_timeout(Duration::from_secs(READ_TIMEOUT_SECS))
        .timeout(Duration::from_secs(TOTAL_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| HobbsError::Rss(format!("failed to create HTTP client: {}", e)))?;

    // Fetch the feed
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| HobbsError::Rss(format!("failed to fetch feed: {}", e)))?;

    // Check response status
    if !response.status().is_success() {
        return Err(HobbsError::Rss(format!(
            "HTTP error: {}",
            response.status()
        )));
    }

    // Check content length if available
    if let Some(content_length) = response.content_length() {
        if content_length > MAX_FEED_SIZE {
            return Err(HobbsError::Rss(format!(
                "feed too large: {} bytes (max {} bytes)",
                content_length, MAX_FEED_SIZE
            )));
        }
    }

    // Read body - reqwest handles streaming internally
    let bytes = response
        .bytes()
        .await
        .map_err(|e| HobbsError::Rss(format!("failed to read response: {}", e)))?;

    // Check actual size
    if bytes.len() as u64 > MAX_FEED_SIZE {
        return Err(HobbsError::Rss(format!(
            "feed too large: {} bytes (max {} bytes)",
            bytes.len(),
            MAX_FEED_SIZE
        )));
    }

    // Parse the feed
    parse_feed(&bytes)
}

/// Validate a URL for SSRF protection.
///
/// This function checks that:
/// - The URL uses http or https scheme
/// - The host is not a private/loopback address
/// - The host is not a reserved hostname
pub fn validate_url(url: &str) -> Result<()> {
    let parsed =
        url::Url::parse(url).map_err(|e| HobbsError::Rss(format!("invalid URL: {}", e)))?;

    // Check scheme
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(HobbsError::Rss(format!(
                "unsupported URL scheme: {}",
                scheme
            )));
        }
    }

    // Get host using the typed host() method
    let host = parsed
        .host()
        .ok_or_else(|| HobbsError::Rss("URL has no host".to_string()))?;

    match host {
        url::Host::Domain(domain) => {
            // Check for forbidden hostnames
            if is_forbidden_hostname(domain) {
                return Err(HobbsError::Rss(format!("forbidden host: {}", domain)));
            }
        }
        url::Host::Ipv4(ipv4) => {
            let ip = IpAddr::V4(ipv4);
            if is_private_ip(&ip) {
                return Err(HobbsError::Rss(format!(
                    "private IP address not allowed: {}",
                    ip
                )));
            }
        }
        url::Host::Ipv6(ipv6) => {
            let ip = IpAddr::V6(ipv6);
            if is_private_ip(&ip) {
                return Err(HobbsError::Rss(format!(
                    "private IP address not allowed: {}",
                    ip
                )));
            }
        }
    }

    Ok(())
}

/// Check if a hostname is forbidden.
fn is_forbidden_hostname(host: &str) -> bool {
    let host_lower = host.to_lowercase();

    // Exact matches
    if host_lower == "localhost" {
        return true;
    }

    // Suffix matches
    let forbidden_suffixes = [
        ".local",
        ".localhost",
        ".internal",
        ".intranet",
        ".corp",
        ".home",
        ".lan",
    ];

    for suffix in &forbidden_suffixes {
        if host_lower.ends_with(suffix) {
            return true;
        }
    }

    false
}

/// Check if an IP address is private/reserved.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            // Loopback: 127.0.0.0/8
            if ipv4.is_loopback() {
                return true;
            }

            // Private: 10.0.0.0/8
            if ipv4.octets()[0] == 10 {
                return true;
            }

            // Private: 172.16.0.0/12
            let octets = ipv4.octets();
            if octets[0] == 172 && (16..=31).contains(&octets[1]) {
                return true;
            }

            // Private: 192.168.0.0/16
            if octets[0] == 192 && octets[1] == 168 {
                return true;
            }

            // Link-local: 169.254.0.0/16
            if octets[0] == 169 && octets[1] == 254 {
                return true;
            }

            // Broadcast
            if ipv4.is_broadcast() {
                return true;
            }

            // Unspecified (0.0.0.0)
            if ipv4.is_unspecified() {
                return true;
            }

            // Documentation: 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
            if (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
            {
                return true;
            }

            false
        }
        IpAddr::V6(ipv6) => {
            // Loopback: ::1
            if ipv6.is_loopback() {
                return true;
            }

            // Unspecified: ::
            if ipv6.is_unspecified() {
                return true;
            }

            // Unique local: fc00::/7
            let segments = ipv6.segments();
            if (segments[0] & 0xfe00) == 0xfc00 {
                return true;
            }

            // Link-local: fe80::/10
            if (segments[0] & 0xffc0) == 0xfe80 {
                return true;
            }

            false
        }
    }
}

/// Parse feed bytes into a ParsedFeed.
fn parse_feed(bytes: &[u8]) -> Result<ParsedFeed> {
    let feed = parser::parse(bytes)
        .map_err(|e| HobbsError::Rss(format!("failed to parse feed: {}", e)))?;

    // Extract title (required)
    let title = feed
        .title
        .map(|t| t.content)
        .unwrap_or_else(|| "Untitled Feed".to_string());

    // Extract description
    let description = feed.description.map(|d| strip_html(&d.content));

    // Extract site URL
    let site_url = feed.links.first().map(|l| l.href.clone());

    // Parse items
    let items: Vec<ParsedItem> = feed
        .entries
        .into_iter()
        .map(|entry| {
            let guid = entry.id;
            let item_title = entry
                .title
                .map(|t| t.content)
                .unwrap_or_else(|| "Untitled".to_string());
            let link = entry.links.first().map(|l| l.href.clone());
            let item_description = entry
                .summary
                .map(|t| t.content)
                .or(entry.content.and_then(|c| c.body))
                .map(|d| truncate_description(&strip_html(&d)));
            let author = entry.authors.first().map(|a| a.name.clone());
            let published_at = entry.published.or(entry.updated);

            ParsedItem {
                guid,
                title: item_title,
                link,
                description: item_description,
                author,
                published_at,
            }
        })
        .collect();

    Ok(ParsedFeed {
        title,
        description,
        site_url,
        items,
    })
}

/// Strip HTML tags from text.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity = String::new();

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '&' if !in_tag => {
                in_entity = true;
                entity.clear();
            }
            ';' if in_entity => {
                in_entity = false;
                // Decode common HTML entities
                match entity.as_str() {
                    "amp" => result.push('&'),
                    "lt" => result.push('<'),
                    "gt" => result.push('>'),
                    "quot" => result.push('"'),
                    "apos" => result.push('\''),
                    "nbsp" => result.push(' '),
                    _ if entity.starts_with('#') => {
                        // Numeric entity
                        if let Some(code) = parse_numeric_entity(&entity) {
                            if let Some(c) = char::from_u32(code) {
                                result.push(c);
                            }
                        }
                    }
                    _ => {
                        // Unknown entity, keep as-is
                        result.push('&');
                        result.push_str(&entity);
                        result.push(';');
                    }
                }
            }
            _ if in_entity => {
                entity.push(ch);
            }
            _ if !in_tag => {
                result.push(ch);
            }
            _ => {}
        }
    }

    // Clean up whitespace
    let result: String = result.split_whitespace().collect::<Vec<&str>>().join(" ");

    result.trim().to_string()
}

/// Parse a numeric HTML entity (e.g., "#123" or "#x7B").
fn parse_numeric_entity(entity: &str) -> Option<u32> {
    if entity.starts_with("#x") || entity.starts_with("#X") {
        // Hexadecimal
        u32::from_str_radix(&entity[2..], 16).ok()
    } else if entity.starts_with('#') {
        // Decimal
        entity[1..].parse().ok()
    } else {
        None
    }
}

/// Truncate description to maximum length.
fn truncate_description(text: &str) -> String {
    if text.len() <= MAX_DESCRIPTION_LENGTH {
        text.to_string()
    } else {
        text.chars().take(MAX_DESCRIPTION_LENGTH).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_valid_https() {
        assert!(validate_url("https://example.com/feed.xml").is_ok());
    }

    #[test]
    fn test_validate_url_valid_http() {
        assert!(validate_url("http://example.com/feed.xml").is_ok());
    }

    #[test]
    fn test_validate_url_invalid_scheme() {
        let result = validate_url("ftp://example.com/feed.xml");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unsupported URL scheme"));
    }

    #[test]
    fn test_validate_url_localhost() {
        let result = validate_url("http://localhost/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("forbidden host"));
    }

    #[test]
    fn test_validate_url_local_domain() {
        let result = validate_url("http://server.local/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("forbidden host"));
    }

    #[test]
    fn test_validate_url_internal_domain() {
        let result = validate_url("http://api.internal/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("forbidden host"));
    }

    #[test]
    fn test_validate_url_loopback_ip() {
        let result = validate_url("http://127.0.0.1/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private IP"));
    }

    #[test]
    fn test_validate_url_private_10() {
        let result = validate_url("http://10.0.0.1/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private IP"));
    }

    #[test]
    fn test_validate_url_private_172() {
        let result = validate_url("http://172.16.0.1/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private IP"));

        // 172.32 should be allowed
        assert!(validate_url("http://172.32.0.1/feed.xml").is_ok());
    }

    #[test]
    fn test_validate_url_private_192() {
        let result = validate_url("http://192.168.1.1/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private IP"));
    }

    #[test]
    fn test_validate_url_link_local() {
        let result = validate_url("http://169.254.1.1/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private IP"));
    }

    #[test]
    fn test_validate_url_ipv6_loopback() {
        let result = validate_url("http://[::1]/feed.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private IP"));
    }

    #[test]
    fn test_is_forbidden_hostname() {
        assert!(is_forbidden_hostname("localhost"));
        assert!(is_forbidden_hostname("server.local"));
        assert!(is_forbidden_hostname("api.localhost"));
        assert!(is_forbidden_hostname("service.internal"));
        assert!(is_forbidden_hostname("corp.intranet"));

        assert!(!is_forbidden_hostname("example.com"));
        assert!(!is_forbidden_hostname("localhost.example.com")); // hostname contains localhost but doesn't end with .localhost
        assert!(!is_forbidden_hostname("news.ycombinator.com"));
    }

    #[test]
    fn test_is_private_ip_v4() {
        // Loopback
        assert!(is_private_ip(&"127.0.0.1".parse().unwrap()));
        assert!(is_private_ip(&"127.255.255.255".parse().unwrap()));

        // Private 10.x
        assert!(is_private_ip(&"10.0.0.1".parse().unwrap()));
        assert!(is_private_ip(&"10.255.255.255".parse().unwrap()));

        // Private 172.16-31.x
        assert!(is_private_ip(&"172.16.0.1".parse().unwrap()));
        assert!(is_private_ip(&"172.31.255.255".parse().unwrap()));
        assert!(!is_private_ip(&"172.32.0.1".parse().unwrap())); // Not private

        // Private 192.168.x
        assert!(is_private_ip(&"192.168.0.1".parse().unwrap()));
        assert!(is_private_ip(&"192.168.255.255".parse().unwrap()));

        // Link-local
        assert!(is_private_ip(&"169.254.1.1".parse().unwrap()));

        // Public
        assert!(!is_private_ip(&"8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip(&"1.1.1.1".parse().unwrap()));
        assert!(!is_private_ip(&"93.184.216.34".parse().unwrap())); // example.com
    }

    #[test]
    fn test_is_private_ip_v6() {
        // Loopback
        assert!(is_private_ip(&"::1".parse().unwrap()));

        // Unspecified
        assert!(is_private_ip(&"::".parse().unwrap()));

        // Link-local
        assert!(is_private_ip(&"fe80::1".parse().unwrap()));

        // Unique local
        assert!(is_private_ip(&"fc00::1".parse().unwrap()));
        assert!(is_private_ip(&"fd00::1".parse().unwrap()));

        // Global
        assert!(!is_private_ip(&"2001:4860:4860::8888".parse().unwrap())); // Google DNS
    }

    #[test]
    fn test_strip_html_basic() {
        assert_eq!(strip_html("<p>Hello</p>"), "Hello");
        assert_eq!(strip_html("<b>Bold</b> text"), "Bold text");
        assert_eq!(strip_html("<div><p>Nested</p></div>"), "Nested");
    }

    #[test]
    fn test_strip_html_entities() {
        assert_eq!(strip_html("&amp;"), "&");
        assert_eq!(strip_html("&lt;tag&gt;"), "<tag>");
        assert_eq!(strip_html("&quot;quoted&quot;"), "\"quoted\"");
        assert_eq!(strip_html("A&nbsp;B"), "A B");
    }

    #[test]
    fn test_strip_html_numeric_entities() {
        assert_eq!(strip_html("&#65;"), "A");
        assert_eq!(strip_html("&#x41;"), "A");
        assert_eq!(strip_html("&#x3042;"), "あ");
    }

    #[test]
    fn test_strip_html_whitespace() {
        assert_eq!(
            strip_html("<p>  Multiple   spaces  </p>"),
            "Multiple spaces"
        );
        assert_eq!(
            strip_html("<p>\n\tNewlines\n\tand\ttabs\n</p>"),
            "Newlines and tabs"
        );
    }

    #[test]
    fn test_truncate_description() {
        let short = "Short text";
        assert_eq!(truncate_description(short), short);

        let exact = "a".repeat(MAX_DESCRIPTION_LENGTH);
        assert_eq!(truncate_description(&exact), exact);

        let long = "a".repeat(MAX_DESCRIPTION_LENGTH + 100);
        let truncated = truncate_description(&long);
        assert_eq!(truncated.len(), MAX_DESCRIPTION_LENGTH);
    }

    #[test]
    fn test_parse_numeric_entity() {
        assert_eq!(parse_numeric_entity("#65"), Some(65));
        assert_eq!(parse_numeric_entity("#x41"), Some(65));
        assert_eq!(parse_numeric_entity("#X41"), Some(65));
        assert_eq!(parse_numeric_entity("#12354"), Some(12354)); // あ
        assert_eq!(parse_numeric_entity("#x3042"), Some(12354)); // あ
        assert_eq!(parse_numeric_entity("invalid"), None);
    }

    #[test]
    fn test_parse_feed_rss() {
        let rss = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <link>https://example.com</link>
    <description>A test feed</description>
    <item>
      <title>First Article</title>
      <link>https://example.com/1</link>
      <guid>guid-1</guid>
      <description>&lt;p&gt;Description&lt;/p&gt;</description>
    </item>
  </channel>
</rss>"#;

        let feed = parse_feed(rss.as_bytes()).unwrap();
        assert_eq!(feed.title, "Test Feed");
        assert_eq!(feed.description, Some("A test feed".to_string()));
        // feed-rs may normalize URLs with trailing slash
        assert!(feed
            .site_url
            .as_ref()
            .unwrap()
            .starts_with("https://example.com"));
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].title, "First Article");
        assert_eq!(feed.items[0].guid, "guid-1");
        assert_eq!(
            feed.items[0].link,
            Some("https://example.com/1".to_string())
        );
        assert_eq!(feed.items[0].description, Some("Description".to_string()));
    }

    #[test]
    fn test_parse_feed_atom() {
        let atom = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Atom Feed</title>
  <link href="https://example.com"/>
  <entry>
    <id>urn:uuid:1</id>
    <title>Atom Entry</title>
    <link href="https://example.com/entry"/>
    <summary>Entry summary</summary>
    <author><name>Author Name</name></author>
    <updated>2025-01-01T00:00:00Z</updated>
  </entry>
</feed>"#;

        let feed = parse_feed(atom.as_bytes()).unwrap();
        assert_eq!(feed.title, "Atom Feed");
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].title, "Atom Entry");
        assert_eq!(feed.items[0].guid, "urn:uuid:1");
        assert_eq!(feed.items[0].author, Some("Author Name".to_string()));
        assert!(feed.items[0].published_at.is_some());
    }

    #[test]
    fn test_parse_feed_minimal() {
        let rss = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <item>
      <guid>1</guid>
    </item>
  </channel>
</rss>"#;

        let feed = parse_feed(rss.as_bytes()).unwrap();
        assert_eq!(feed.title, "Untitled Feed");
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].title, "Untitled");
        assert_eq!(feed.items[0].guid, "1");
    }

    #[test]
    fn test_parse_feed_invalid() {
        let invalid = "This is not XML";
        assert!(parse_feed(invalid.as_bytes()).is_err());
    }
}
