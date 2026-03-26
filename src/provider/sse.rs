use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};

use super::error::ProviderError;

pub struct ProviderStream {
    pub content_type: String,
    pub events: BoxStream<'static, Result<ProviderStreamEvent, ProviderError>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderStreamEvent {
    Data(String),
    JsonDelta(serde_json::Value),
    Comment(String),
    Retry(u64),
    Done,
}

pub fn parse_sse_text(text: &str) -> Vec<Result<ProviderStreamEvent, ProviderError>> {
    let mut events = Vec::new();

    for frame in text.split("\n\n") {
        let mut data_lines: Vec<String> = Vec::new();
        let mut has_event = false;
        for line in frame.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            if let Some(comment) = line.strip_prefix(':') {
                events.push(Ok(ProviderStreamEvent::Comment(comment.trim().to_string())));
                has_event = true;
                continue;
            }
            if let Some(retry) = line.strip_prefix("retry:") {
                if let Ok(value) = retry.trim().parse::<u64>() {
                    events.push(Ok(ProviderStreamEvent::Retry(value)));
                }
                has_event = true;
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data.is_empty() {
                    continue;
                }
                has_event = true;
                data_lines.push(data.to_string());
                continue;
            }
            if let Some(event_name) = line.strip_prefix("event:") {
                has_event = true;
                let name = event_name.trim();
                if name == "error" {
                    events.push(Ok(ProviderStreamEvent::Comment("event:error".to_string())));
                }
            }
        }

        if let Some(data) = (!data_lines.is_empty()).then(|| data_lines.join("\n")) {
            if data == "[DONE]" {
                events.push(Ok(ProviderStreamEvent::Done));
            } else if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) {
                events.push(Ok(ProviderStreamEvent::JsonDelta(value)));
            } else {
                events.push(Ok(ProviderStreamEvent::Data(data)));
            }
        }

        if !has_event && !frame.trim().is_empty() {
            events.push(Ok(ProviderStreamEvent::Data(frame.trim().to_string())));
        }
    }

    if events.is_empty() {
        events.push(Ok(ProviderStreamEvent::Data(text.to_string())));
    }

    events.push(Ok(ProviderStreamEvent::Done));
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_text_json_data_and_done() {
        let text = "data: {\"delta\":\"hello\"}\n\ndata: [DONE]\n\n";
        let events = parse_sse_text(text);

        assert!(matches!(events[0], Ok(ProviderStreamEvent::JsonDelta(_))));
        assert!(matches!(events[1], Ok(ProviderStreamEvent::Done)));
        assert!(matches!(events.last(), Some(Ok(ProviderStreamEvent::Done))));
    }

    #[test]
    fn test_parse_sse_text_comment_and_retry() {
        let text = ": keepalive\nretry: 1500\n\n";
        let events = parse_sse_text(text);

        assert_eq!(
            events[0],
            Ok(ProviderStreamEvent::Comment("keepalive".to_string()))
        );
        assert_eq!(events[1], Ok(ProviderStreamEvent::Retry(1500)));
        assert!(matches!(events.last(), Some(Ok(ProviderStreamEvent::Done))));
    }

    #[test]
    fn test_parse_sse_text_plain_fallback() {
        let text = "plain text payload";
        let events = parse_sse_text(text);

        assert_eq!(
            events[0],
            Ok(ProviderStreamEvent::Data("plain text payload".to_string()))
        );
        assert!(matches!(events.last(), Some(Ok(ProviderStreamEvent::Done))));
    }
}
