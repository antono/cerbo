//! Rewriting a note body's links to `cerbo://` (design D4). Only link syntax is
//! touched, and never inside code.

use super::markdown;
use super::resolve::{Findings, LinkIndex, NoteEntry, Target};
use regex::{Captures, Regex};
use std::sync::LazyLock;

/// A wikilink or embed, or an inline Markdown link or image. One alternation,
/// so the two never overlap.
static LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?P<wl>(?P<wl_bang>!?)\[\[(?P<wl_inner>[^\[\]\n]+?)\]\])",
        // Link text may hold one level of brackets, a wikilink included.
        r"|(?P<md>(?P<md_bang>!?)\[(?P<md_text>(?:[^\[\]\n\\]|\\.|\[(?:[^\[\]\n]|\[[^\[\]\n]*\])*\])*)\]",
        r"\((?P<md_dest><[^<>\n]*>|(?:[^()\s\\]|\\.|\([^()\s]*\))+)",
        r#"(?P<md_title>\s+(?:"[^"\n]*"|'[^'\n]*'|\([^()\n]*\)))?\s*\))"#,
    ))
    .expect("link pattern is valid")
});

/// `[[target]]` occurrences in a string, for frontmatter values.
pub static WIKILINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\[\]\n]+?)\]\]").expect("wikilink pattern is valid"));

/// An Obsidian embed size such as `300` or `300x200`, which is not alt text.
static EMBED_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+(x\d+)?$").expect("size pattern is valid"));

static URL_SCHEME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9+.\-]*:").expect("scheme pattern is valid"));

/// The parts of a wikilink's inner text `target#heading|alias`.
pub struct WikiParts<'a> {
    pub target: &'a str,
    pub fragment: Option<&'a str>,
    pub alias: Option<&'a str>,
}

pub fn wiki_parts(inner: &str) -> WikiParts<'_> {
    let (target, alias) = match inner.split_once('|') {
        // In a table, Obsidian escapes the pipe: `[[x\|alias]]`.
        Some((t, a)) => (
            t.strip_suffix('\\').unwrap_or(t),
            Some(a.trim()).filter(|a| !a.is_empty()),
        ),
        None => (inner, None),
    };
    let (target, fragment) = match target.split_once('#') {
        Some((t, f)) => (t, Some(f)),
        None => (target, None),
    };
    WikiParts {
        target: target.trim(),
        fragment,
        alias,
    }
}

/// Link text with the characters that would end it escaped.
pub fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `%20` and friends decoded; the input unchanged when it does not decode to UTF-8.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
            && let Some(b) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// Rewrite every resolvable link in `body`, a body of `note`. Unresolved links
/// stay as written and are recorded in `findings`.
pub fn rewrite_body(
    body: &str,
    note: &NoteEntry,
    index: &LinkIndex,
    findings: &mut Findings,
) -> String {
    let code = markdown::code_ranges(body);
    let mut out = String::with_capacity(body.len());
    let mut last = 0;
    for caps in LINK.captures_iter(body) {
        let whole = caps.get(0).expect("group 0 always matches");
        if markdown::in_ranges(&code, whole.start()) || escaped(body, whole.start()) {
            continue;
        }
        let replacement = if caps.name("wl").is_some() {
            wikilink(&caps, note, index, findings)
        } else {
            markdown_link(&caps, note, index, findings)
        };
        if let Some(replacement) = replacement {
            out.push_str(&body[last..whole.start()]);
            out.push_str(&replacement);
            last = whole.end();
        }
    }
    out.push_str(&body[last..]);
    out
}

/// Whether the character at `at` is escaped: an odd number of backslashes
/// right before it.
fn escaped(body: &str, at: usize) -> bool {
    body.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 1
}

fn wikilink(
    caps: &Captures,
    note: &NoteEntry,
    index: &LinkIndex,
    findings: &mut Findings,
) -> Option<String> {
    let original = &caps[0];
    let embed = !caps["wl_bang"].is_empty();
    let parts = wiki_parts(&caps["wl_inner"]);
    // `[[#Heading]]` points into the note itself; there is nothing to rewrite.
    if parts.target.is_empty() {
        return None;
    }
    let Some(res) = index.wikilink(parts.target, &note.folder) else {
        findings.unresolved(&note.rel, original, None);
        return None;
    };
    findings.resolved(index, &note.rel, original, &res);
    findings.links += 1;
    if parts.fragment.is_some() {
        findings.dropped_fragments += 1;
    }
    let uuid = index.uuid_of(res.target);
    Some(match res.target {
        // Transclusion is out of scope: an embedded note is an ordinary link.
        Target::Note(i) => {
            let text = parts.alias.unwrap_or(&index.notes[i].title);
            format!("[{}](cerbo://{uuid})", escape_text(text))
        }
        Target::File(i) => {
            let alt = parts
                .alias
                .filter(|a| !EMBED_SIZE.is_match(a))
                .unwrap_or(&index.files[i].name);
            let bang = if embed { "!" } else { "" };
            format!("{bang}[{}](cerbo://{uuid})", escape_text(alt))
        }
    })
}

fn markdown_link(
    caps: &Captures,
    note: &NoteEntry,
    index: &LinkIndex,
    findings: &mut Findings,
) -> Option<String> {
    let raw = &caps["md_dest"];
    let dest = raw
        .strip_prefix('<')
        .and_then(|d| d.strip_suffix('>'))
        .unwrap_or(raw);
    if dest.is_empty() || dest.starts_with('#') || URL_SCHEME.is_match(dest) {
        return None;
    }
    // Split before decoding: `%23` and `%3F` belong to the file name.
    let (path, fragment) = match dest.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (dest, None),
    };
    let path = percent_decode(path.split_once('?').map_or(path, |(p, _)| p));
    let original = &caps[0];
    let Some(res) = index.markdown(&path, &note.folder) else {
        findings.unresolved(&note.rel, original, None);
        return None;
    };
    findings.resolved(index, &note.rel, original, &res);
    findings.links += 1;
    if fragment.is_some() {
        findings.dropped_fragments += 1;
    }
    let bang = match res.target {
        Target::File(_) => &caps["md_bang"],
        Target::Note(_) => "",
    };
    let title = caps.name("md_title").map_or("", |m| m.as_str());
    // A link cannot hold a link: a wikilink in the text becomes its text.
    let text = WIKILINK.replace_all(&caps["md_text"], |c: &Captures| {
        let parts = wiki_parts(&c[1]);
        escape_text(parts.alias.unwrap_or(parts.target))
    });
    Some(format!(
        "{bang}[{text}](cerbo://{}{title})",
        index.uuid_of(res.target)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault_import::resolve::FileEntry;

    fn note(rel: &str, title: &str) -> NoteEntry {
        let folder = rel
            .rsplit_once('/')
            .map(|(f, _)| f)
            .unwrap_or("")
            .to_string();
        NoteEntry {
            rel: rel.into(),
            folder,
            title: title.into(),
            uuid: format!("n-{}", title.replace(' ', "_")),
        }
    }

    fn file(rel: &str) -> FileEntry {
        let (folder, name) = rel.rsplit_once('/').unwrap_or(("", rel));
        FileEntry {
            rel: rel.into(),
            folder: folder.into(),
            name: name.into(),
            uuid: format!("f-{name}"),
        }
    }

    fn index() -> LinkIndex {
        LinkIndex::new(
            vec![
                note(
                    "orange-pi-zero-2w-2g-octoprint.md",
                    "Orange PI Zero 2W 2G - Octoprint",
                ),
                note("desktop-pc.md", "Desktop PC"),
                note("devices.md", "Devices"),
                note("notes/Other Note.md", "Other Note"),
                note("journals/2025-04-27.md", "2025-04-27"),
            ],
            &[],
            vec![
                file("attachments/photo.png"),
                file("attachments/pdf/BIGTREETECH-Octopus-1.1-color-PIN.pdf"),
            ],
        )
    }

    fn run(body: &str, from: &str) -> (String, Findings) {
        let ix = index();
        let mut findings = Findings::default();
        let from = note(from, "From");
        let out = rewrite_body(body, &from, &ix, &mut findings);
        (out, findings)
    }

    #[test]
    fn link_by_file_stem_uses_title() {
        let (out, f) = run("- [[orange-pi-zero-2w-2g-octoprint]]\n", "x.md");
        assert_eq!(
            out,
            "- [Orange PI Zero 2W 2G - Octoprint](cerbo://n-Orange_PI_Zero_2W_2G_-_Octoprint)\n"
        );
        assert_eq!(f.links, 1);
    }

    #[test]
    fn link_by_title_with_alias() {
        let (out, _) = run("[[Desktop PC|my PC]]", "x.md");
        assert_eq!(out, "[my PC](cerbo://n-Desktop_PC)");
    }

    #[test]
    fn heading_fragment_is_dropped_and_counted() {
        let (out, f) = run("[[Desktop PC#Specs]]", "x.md");
        assert_eq!(out, "[Desktop PC](cerbo://n-Desktop_PC)");
        assert_eq!(f.dropped_fragments, 1);
    }

    #[test]
    fn unresolved_is_kept_and_reported() {
        let (out, f) = run("see [[Missing Note]] now", "x.md");
        assert_eq!(out, "see [[Missing Note]] now");
        assert_eq!(f.unresolved.len(), 1);
        assert_eq!(f.unresolved[0].link, "[[Missing Note]]");
        assert_eq!(f.unresolved[0].note, "x.md");
    }

    #[test]
    fn note_embed_becomes_plain_link() {
        let (out, _) = run("![[Other Note]]", "x.md");
        assert_eq!(out, "[Other Note](cerbo://n-Other_Note)");
    }

    #[test]
    fn wikilinks_in_code_are_untouched() {
        let body = "```\n[[Desktop PC]]\n```\n\nand `[[Desktop PC]]` inline\n";
        let (out, f) = run(body, "x.md");
        assert_eq!(out, body);
        assert_eq!(f.links, 0);
        assert!(f.unresolved.is_empty());
    }

    #[test]
    fn relative_markdown_note_link() {
        let (out, _) = run("[see](../devices.md)", "journals/2025-04-27.md");
        assert_eq!(out, "[see](cerbo://n-Devices)");
        let (out, _) = run("[o](notes/Other%20Note.md)", "x.md");
        assert_eq!(out, "[o](cerbo://n-Other_Note)");
        let (out, _) = run("[o](<notes/Other Note.md#Part> \"tip\")", "x.md");
        assert_eq!(out, "[o](cerbo://n-Other_Note \"tip\")");
    }

    #[test]
    fn external_urls_are_untouched() {
        let body = "[ex](https://example.com/a.md) [m](mailto:a@example.com) [h](#top)";
        let (out, f) = run(body, "x.md");
        assert_eq!(out, body);
        assert!(f.unresolved.is_empty());
    }

    #[test]
    fn obsidian_image_embed() {
        let (out, f) = run("![[photo.png]] and ![[photo.png|300]]", "notes/a.md");
        assert_eq!(
            out,
            "![photo.png](cerbo://f-photo.png) and ![photo.png](cerbo://f-photo.png)"
        );
        assert_eq!(f.used_files, [0], "one file, used twice");
    }

    #[test]
    fn tolaria_attachment_link() {
        let (out, f) = run(
            "[manual](attachments/pdf/BIGTREETECH-Octopus-1.1-color-PIN.pdf)",
            "x.md",
        );
        assert_eq!(
            out,
            "[manual](cerbo://f-BIGTREETECH-Octopus-1.1-color-PIN.pdf)"
        );
        assert_eq!(f.used_files, [1]);
        let (out, _) = run("![](attachments/photo.png)", "x.md");
        assert_eq!(out, "![](cerbo://f-photo.png)");
    }

    #[test]
    fn broken_local_markdown_link_is_reported() {
        let (out, f) = run("[gone](missing.pdf)", "x.md");
        assert_eq!(out, "[gone](missing.pdf)");
        assert_eq!(f.unresolved[0].link, "[gone](missing.pdf)");
    }

    #[test]
    fn inline_tags_stay_text() {
        let (out, _) = run("#rust is fun", "x.md");
        assert_eq!(out, "#rust is fun");
    }

    #[test]
    fn title_with_brackets_is_escaped() {
        assert_eq!(escape_text("a [b]"), "a \\[b\\]");
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(percent_decode("Other%20Note.md"), "Other Note.md");
        assert_eq!(percent_decode("%D1%84.md"), "ф.md");
        assert_eq!(percent_decode("100%"), "100%");
        // Only two hex digits make an escape; `u8::from_str_radix` takes a sign.
        assert_eq!(percent_decode("a%+5b.md"), "a%+5b.md");
    }

    #[test]
    fn encoded_hash_and_question_mark_belong_to_the_name() {
        let ix = LinkIndex::new(
            vec![
                note("C# basics.md", "C# basics"),
                note("Why?.md", "Why"),
                note("C.md", "C"),
            ],
            &[],
            vec![],
        );
        let mut f = Findings::default();
        let from = note("x.md", "From");
        let out = rewrite_body("[c](C%23%20basics.md) [q](Why%3F.md)", &from, &ix, &mut f);
        assert_eq!(out, "[c](cerbo://n-C#_basics) [q](cerbo://n-Why)");
        assert_eq!(f.dropped_fragments, 0);
        assert!(f.unresolved.is_empty());
    }

    #[test]
    fn escaped_brackets_are_not_links() {
        let body = "\\[[Desktop PC]] and \\[text](devices.md)";
        let (out, f) = run(body, "x.md");
        assert_eq!(out, body);
        assert_eq!(f.links, 0);
        assert!(f.unresolved.is_empty());
        // An escaped backslash does not escape the link.
        let (out, f) = run("\\\\[[Desktop PC]]", "x.md");
        assert_eq!(out, "\\\\[Desktop PC](cerbo://n-Desktop_PC)");
        assert_eq!(f.links, 1);
    }

    #[test]
    fn wikilink_in_link_text_is_not_nested() {
        let (out, f) = run("[see [[Desktop PC]]](devices.md)", "x.md");
        assert_eq!(out, "[see Desktop PC](cerbo://n-Devices)");
        assert_eq!(f.links, 1);
        let (out, f) = run("[see [[Desktop PC]]](missing.md)", "x.md");
        assert_eq!(out, "[see [[Desktop PC]]](missing.md)");
        assert_eq!(f.unresolved[0].link, "[see [[Desktop PC]]](missing.md)");
    }

    #[test]
    fn markdown_link_by_shortest_path() {
        let (out, f) = run("[x](Other%20Note.md)", "x.md");
        assert_eq!(out, "[x](cerbo://n-Other_Note)");
        assert!(f.unresolved.is_empty());
    }
}
