//! Safe, presentation-neutral helpers for note content.

use url::Url;

/// Return a safe YouTube embed URL while leaving the original note URL intact.
///
/// Notes are stored as text. A future UI can use this derived value for an iframe
/// or native player after applying its own sandbox policy; arbitrary pasted markup
/// is never interpreted by the core.
pub fn youtube_embed_url(original: &str) -> Option<String> {
    let url = Url::parse(original).ok()?;
    if url.scheme() != "https" {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let video_id = match host.as_str() {
        "youtube.com" | "www.youtube.com" => {
            if url.path() != "/watch" {
                return None;
            }
            url.query_pairs()
                .find(|(key, _)| key == "v")
                .map(|(_, value)| value.into_owned())?
        }
        "youtu.be" => url.path().trim_matches('/').to_string(),
        _ => return None,
    };
    if video_id.is_empty()
        || video_id.len() > 64
        || !video_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return None;
    }
    Some(format!("https://www.youtube.com/embed/{video_id}"))
}

#[cfg(test)]
mod tests {
    use super::youtube_embed_url;

    #[test]
    fn recognizes_supported_youtube_forms_without_executing_markup() {
        assert_eq!(
            youtube_embed_url("https://www.youtube.com/watch?v=abc_123"),
            Some("https://www.youtube.com/embed/abc_123".into())
        );
        assert_eq!(
            youtube_embed_url("https://youtu.be/abc-123"),
            Some("https://www.youtube.com/embed/abc-123".into())
        );
        assert_eq!(youtube_embed_url("javascript:alert(1)"), None);
        assert_eq!(
            youtube_embed_url("https://example.test/watch?v=abc_123"),
            None
        );
    }
}
