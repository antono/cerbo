//! What the importer needs to know about a note's Markdown structure: where
//! code is (so links inside it are left alone, design D4) and its H1 headings.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::ops::Range;

fn parser(body: &str) -> Parser<'_> {
    Parser::new_ext(
        body,
        Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH,
    )
}

/// Byte ranges of fenced and indented code blocks and inline code spans.
pub fn code_ranges(body: &str) -> Vec<Range<usize>> {
    parser(body)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            // A Start event's range spans the whole element.
            Event::Start(Tag::CodeBlock(_)) | Event::Code(_) => Some(range),
            _ => None,
        })
        .collect()
}

/// Whether byte `offset` falls inside any of `ranges`.
pub fn in_ranges(ranges: &[Range<usize>], offset: usize) -> bool {
    ranges.iter().any(|r| r.contains(&offset))
}

/// The text of the first level-1 heading anywhere in `body`, trimmed; `None`
/// when there is none or it is empty.
pub fn first_h1(body: &str) -> Option<String> {
    let mut events = parser(body);
    while let Some(event) = events.next() {
        if let Event::Start(Tag::Heading {
            level: HeadingLevel::H1,
            ..
        }) = event
        {
            let text = heading_text(&mut events);
            return (!text.is_empty()).then_some(text);
        }
    }
    None
}

/// When `body`'s first block is a level-1 heading: its text and the byte
/// offset where the heading block ends.
pub fn leading_h1(body: &str) -> Option<(String, usize)> {
    let mut events = parser(body).into_offset_iter();
    let (event, range) = events.next()?;
    if !matches!(
        event,
        Event::Start(Tag::Heading {
            level: HeadingLevel::H1,
            ..
        })
    ) {
        return None;
    }
    let text = heading_text(&mut events.by_ref().map(|(e, _)| e));
    Some((text, range.end))
}

fn heading_text<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> String {
    let mut text = String::new();
    for event in events {
        match event {
            Event::End(TagEnd::Heading(_)) => break,
            Event::Text(t) | Event::Code(t) => text.push_str(&t),
            _ => {}
        }
    }
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_fenced_indented_and_inline_code() {
        let body = "a [[x]]\n\n```\n[[y]]\n```\n\nsee `[[z]]` here\n\n    [[w]]\n";
        let ranges = code_ranges(body);
        let at = |needle: &str| body.find(needle).unwrap();
        assert!(!in_ranges(&ranges, at("[[x]]")));
        assert!(in_ranges(&ranges, at("[[y]]")));
        assert!(in_ranges(&ranges, at("[[z]]")));
        assert!(in_ranges(&ranges, at("[[w]]")));
    }

    #[test]
    fn first_and_leading_h1() {
        assert_eq!(first_h1("intro\n\n# Devices\n"), Some("Devices".into()));
        assert_eq!(first_h1("## Only H2\n"), None);
        assert_eq!(
            leading_h1("\n# Devices\n\nbody"),
            Some(("Devices".into(), 11))
        );
        assert_eq!(leading_h1("intro\n\n# Devices\n"), None);
        // Setext headings count too.
        assert_eq!(first_h1("Title\n=====\n"), Some("Title".into()));
    }
}
