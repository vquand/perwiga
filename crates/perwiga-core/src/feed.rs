//! Standards-compliant RSS and Atom normalization owned by the shared core.

use quick_xml::{events::Event, Reader};

use crate::{
    error::{PerwigaError, Result},
    model::FeedItem,
};

/// A parsed feed record before it is assigned a database identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFeedItem {
    pub external_identity: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub published_at: Option<String>,
}

/// Parse the common RSS 2.0 and Atom fields needed by the normalized feed model.
///
/// The parser deliberately treats source text as untrusted: unknown elements are
/// ignored, URLs are retained as text for later boundary validation, and missing
/// identities fall back to a canonical URL before the caller persists a record.
pub fn parse_feed(xml: &str) -> Result<Vec<ParsedFeedItem>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut items = Vec::new();
    let mut current: Option<ParsedFeedItem> = None;
    let mut current_field: Option<Vec<u8>> = None;
    let mut in_item = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let name = event.name().as_ref().to_vec();
                if name == b"item" || name == b"entry" {
                    if current.is_some() {
                        return Err(PerwigaError::FeedParse("nested feed item".into()));
                    }
                    current = Some(ParsedFeedItem {
                        external_identity: None,
                        title: String::new(),
                        url: None,
                        published_at: None,
                    });
                    in_item = true;
                    current_field = None;
                } else if in_item
                    && matches!(
                        name.as_slice(),
                        b"title"
                            | b"id"
                            | b"guid"
                            | b"link"
                            | b"pubDate"
                            | b"published"
                            | b"updated"
                    )
                {
                    current_field = Some(name);
                } else {
                    current_field = None;
                }
            }
            Ok(Event::Empty(event)) if in_item && event.name().as_ref() == b"link" => {
                if let Some(item) = current.as_mut() {
                    for attribute in event.attributes().flatten() {
                        if attribute.key.as_ref() == b"href" {
                            item.url = Some(String::from_utf8_lossy(&attribute.value).into_owned());
                        }
                    }
                }
            }
            Ok(Event::Text(text)) if in_item => {
                let value = text
                    .unescape()
                    .map_err(|error| PerwigaError::FeedParse(error.to_string()))?
                    .into_owned();
                let Some(field) = current_field.as_deref() else {
                    buffer.clear();
                    continue;
                };
                let Some(item) = current.as_mut() else {
                    buffer.clear();
                    continue;
                };
                match field {
                    b"title" => item.title = value,
                    b"id" | b"guid" => item.external_identity = Some(value),
                    b"link" => item.url = Some(value),
                    b"pubDate" | b"published" | b"updated" => item.published_at = Some(value),
                    _ => {}
                }
            }
            Ok(Event::End(event)) => {
                let name = event.name().as_ref().to_vec();
                if name == b"item" || name == b"entry" {
                    if let Some(item) = current.take() {
                        let title = item.title.trim().to_string();
                        if title.is_empty() {
                            return Err(PerwigaError::FeedParse(
                                "feed item is missing a title".into(),
                            ));
                        }
                        items.push(item);
                    }
                    in_item = false;
                }
                current_field = None;
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(PerwigaError::FeedParse(error.to_string())),
            _ => {}
        }
        buffer.clear();
    }
    Ok(items)
}

pub fn normalize_feed_items(
    parsed: &[ParsedFeedItem],
    discovered_at: &str,
    provenance: &str,
) -> Result<Vec<FeedItem>> {
    if discovered_at.trim().is_empty() || provenance.trim().is_empty() {
        return Err(PerwigaError::Validation(
            "feed discovery time and provenance are required".into(),
        ));
    }
    chrono::DateTime::parse_from_rfc3339(discovered_at).map_err(|error| {
        PerwigaError::Validation(format!("feed discovery time must be RFC3339: {error}"))
    })?;
    parsed
        .iter()
        .map(|item| {
            let identity = item
                .external_identity
                .clone()
                .or_else(|| item.url.clone())
                .ok_or_else(|| {
                    PerwigaError::FeedParse(format!(
                        "feed item {:?} has no GUID, id, or URL",
                        item.title
                    ))
                })?;
            if item.title.trim().is_empty() {
                return Err(PerwigaError::FeedParse(
                    "feed item is missing a title".into(),
                ));
            }
            Ok(FeedItem {
                id: uuid::Uuid::new_v4().simple().to_string(),
                source_id: String::new(),
                external_identity: identity,
                title: item.title.trim().to_string(),
                url: item.url.clone(),
                published_at: item.published_at.clone(),
                discovered_at: discovered_at.to_string(),
                is_read: false,
                item_kind: None,
                provenance: provenance.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rss_guid_and_falls_back_to_url_when_guid_is_missing() {
        let xml = r#"<rss><channel>
          <item><guid>chapter-1</guid><title>第一章</title><link>https://example.test/1</link></item>
          <item><title>Chapter 2</title><link>https://example.test/2</link></item>
        </channel></rss>"#;
        let parsed = parse_feed(xml).expect("valid RSS fixture");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].external_identity.as_deref(), Some("chapter-1"));
        assert_eq!(parsed[1].external_identity, None);
        let normalized =
            normalize_feed_items(&parsed, "2026-08-22T00:00:00Z", "fixture").expect("normalizes");
        assert_eq!(normalized[1].external_identity, "https://example.test/2");
        assert_eq!(normalized[0].title, "第一章");
    }

    #[test]
    fn parses_atom_link_attributes() {
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom">
          <entry><id>a-1</id><title>Atom title</title><link href="https://example.test/a"/><updated>2026-08-22T00:00:00Z</updated></entry>
        </feed>"#;
        let parsed = parse_feed(xml).expect("valid Atom fixture");
        assert_eq!(parsed[0].external_identity.as_deref(), Some("a-1"));
        assert_eq!(parsed[0].url.as_deref(), Some("https://example.test/a"));
    }
}
