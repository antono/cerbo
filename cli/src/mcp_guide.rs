//! The guides an agent gets for working with a vault through `cerbo mcp`.
//!
//! Three topics, one source file each under `guide/`: `usage` (finding and
//! reading), `management` (changing the vault) and `ontology` (types,
//! vocabularies and the RDF graph). Each is rendered for what the session may
//! do: sections between `<!--if write-->`, `<!--if not write-->` and
//! `<!--if host-->` and `<!--end-->` appear only when the vault lets the agent
//! change it, or host access is on. A tool is written `{{tool:name}}` and comes
//! out as `` `name` ``, so a test can check every tool a guide names exists.
//!
//! The server offers them as the `guide` tool (always, whatever the client
//! supports) and as the resources `cerbo://guide/<topic>`. The same texts, with
//! every section, are the skill files `.agents/skills/cerbo-mcp-<topic>/SKILL.md`
//! for agents that run outside the daemon's sandbox.

/// One guide.
pub struct Topic {
    /// The `topic` argument of the guide tool and the end of the resource URI.
    pub id: &'static str,
    pub title: &'static str,
    /// One sentence for resource lists and the tool's help.
    pub about: &'static str,
    source: &'static str,
    /// For the skill file's front matter.
    #[cfg_attr(not(test), allow(dead_code))]
    skill_description: &'static str,
}

pub const TOPICS: [Topic; 3] = [
    Topic {
        id: "usage",
        title: "Using a Cerbo vault",
        about: "Finding and reading: which search to use (by meaning, text, SPARQL, links) and how to read the results.",
        source: include_str!("guide/usage.md"),
        skill_description: "Use when finding or reading notes in a Cerbo vault through the cerbo MCP tools: search by \
meaning, by text or with SPARQL, following links, reading scores. Examples: \\\"Find my notes about Berlin\\\", \
\\\"Which pages link here?\\\"",
    },
    Topic {
        id: "management",
        title: "Managing the vault",
        about: "Changing the vault: safe editing with base_version, creating, moving, renaming, linking, deleting, approvals, files outside the vault.",
        source: include_str!("guide/management.md"),
        skill_description: "Use when changing a Cerbo vault through the cerbo MCP tools: creating, editing, moving, \
renaming, linking and deleting pages safely with base_version, and when changes wait for approval. Examples: \
\\\"Add a page for this meeting\\\", \\\"Move these notes into a folder\\\"",
    },
    Topic {
        id: "ontology",
        title: "Ontologies, vocabularies and the RDF graph",
        about: "Types and prefixes, the known ontologies, what the RDF graph holds, useful SPARQL questions, creating new types.",
        source: include_str!("guide/ontology.md"),
        skill_description: "Use when annotating text with types in a Cerbo vault, choosing among known ontologies \
(schema, foaf, dcterms, skos), querying the RDF graph with SPARQL, or adding a new type to the vault's own vocabulary. \
Examples: \\\"Annotate the people in this page\\\", \\\"There is no type for recipes, add one\\\", \\\"Which \
annotations are broken?\\\"",
    },
];

/// The topic named `id`.
pub fn topic(id: &str) -> Option<&'static Topic> {
    TOPICS.iter().find(|t| t.id == id)
}

/// The topic ids, for messages: `usage, management, ontology`.
pub fn ids() -> String {
    TOPICS.iter().map(|t| t.id).collect::<Vec<_>>().join(", ")
}

/// The URI of a topic as a resource.
pub fn uri(topic: &Topic) -> String {
    format!("cerbo://guide/{}", topic.id)
}

/// The topic a resource URI names.
pub fn topic_of_uri(uri: &str) -> Option<&'static Topic> {
    uri.strip_prefix("cerbo://guide/").and_then(topic)
}

impl Topic {
    /// This guide for a session that can (or cannot) change the vault and may
    /// (or may not) use host access.
    pub fn render(&self, write: bool, host: bool) -> String {
        let mut out = String::new();
        let mut skip: Vec<bool> = Vec::new();
        for line in self.source.lines() {
            match line.trim() {
                "<!--if write-->" => skip.push(!write),
                "<!--if not write-->" => skip.push(write),
                "<!--if host-->" => skip.push(!host),
                "<!--end-->" => {
                    skip.pop();
                }
                _ if skip.iter().any(|s| *s) => {}
                _ => {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        replace_tools(&out)
    }

    /// Every tool name the source mentions, in order of appearance.
    #[cfg(test)]
    pub fn tools_named(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut rest = self.source;
        while let Some(start) = rest.find("{{tool:") {
            let after = &rest[start + "{{tool:".len()..];
            let end = after.find("}}").expect("an unclosed {{tool:..}} in a guide");
            let name = after[..end].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
            rest = &after[end + 2..];
        }
        names
    }

    /// The skill file for agents outside the sandbox: front matter and every section.
    #[cfg(test)]
    pub fn skill_file(&self) -> String {
        format!(
            "---\nname: cerbo-mcp-{}\ndescription: \"{}\"\n---\n\n{}",
            self.id,
            self.skill_description,
            self.render(true, true)
        )
    }
}

/// `{{tool:name}}` to `` `name` ``.
fn replace_tools(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{tool:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "{{tool:".len()..];
        let end = after.find("}}").expect("an unclosed {{tool:..}} in a guide");
        out.push('`');
        out.push_str(&after[..end]);
        out.push('`');
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// The summary in the server instructions: enough to work, and where the rest is.
pub const SUMMARY: &str = "To find notes: search_semantic when you know the meaning but not the words (any language), \
search_text for exact words, sparql_query for structured questions. Annotations look like [Text]{prefix:Type}; check \
ontology_list and ontology_types before choosing a type. Three guides explain the rest: call the `guide` tool with \
topic `usage` (finding and reading), `management` (changing the vault) or `ontology` (types, vocabularies, the RDF \
graph); they are also the resources cerbo://guide/usage, cerbo://guide/management and cerbo://guide/ontology.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_only_session_is_told_about_no_tool_that_changes_anything() {
        for topic in &TOPICS {
            let text = topic.render(false, false);
            for tool in crate::mcp::WRITE_TOOLS.iter().chain(crate::mcp::HOST_TOOLS.iter()) {
                assert!(!text.contains(tool), "the read-only {} guide names {tool}", topic.id);
            }
        }
        assert!(topic("usage").unwrap().render(false, false).contains("`search_semantic`"));
        assert!(topic("ontology").unwrap().render(false, false).contains("`ontology_types`"));
        assert!(topic("management").unwrap().render(false, false).contains("read-only for you"));
    }

    #[test]
    fn write_and_host_sections_appear_only_when_allowed() {
        let management = topic("management").unwrap();
        assert!(management.render(true, false).contains("`edit_page`"));
        assert!(!management.render(true, false).contains("read-only for you"));
        assert!(!management.render(true, false).contains("host_read"));
        assert!(management.render(true, true).contains("`host_read`"));
        assert!(management.render(false, true).contains("`host_read`"));
        assert!(!management.render(false, true).contains("edit_page"));
        let ontology = topic("ontology").unwrap();
        assert!(ontology.render(true, false).contains("`ontology_add_type`"));
        assert!(ontology.render(false, false).contains("cannot add types"));
        assert!(!ontology.render(true, false).contains("cannot add types"));
    }

    #[test]
    fn no_marker_is_left_in_the_output() {
        for topic in &TOPICS {
            for (w, h) in [(false, false), (true, false), (false, true), (true, true)] {
                let text = topic.render(w, h);
                assert!(!text.contains("{{") && !text.contains("<!--"), "{} {w} {h}: {text}", topic.id);
            }
        }
    }

    #[test]
    fn every_tool_a_guide_names_is_registered() {
        let registered = crate::mcp::tool_names();
        for topic in &TOPICS {
            for name in topic.tools_named() {
                assert!(registered.contains(&name), "the {} guide names `{name}`, which the server does not offer", topic.id);
            }
        }
    }

    #[test]
    fn the_summary_names_real_tools_and_every_topic() {
        let registered = crate::mcp::tool_names();
        for word in SUMMARY.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            if word.contains('_') {
                assert!(registered.iter().any(|t| t == word), "the summary names `{word}`");
            }
        }
        for topic in &TOPICS {
            assert!(SUMMARY.contains(&format!("`{}`", topic.id)), "the summary does not mention {}", topic.id);
            assert!(SUMMARY.contains(&uri(topic)), "the summary does not list {}", uri(topic));
        }
    }

    #[test]
    fn topics_are_found_by_id_and_by_uri() {
        assert_eq!(topic("ontology").unwrap().title, "Ontologies, vocabularies and the RDF graph");
        assert!(topic("nope").is_none());
        assert_eq!(topic_of_uri("cerbo://guide/management").unwrap().id, "management");
        assert!(topic_of_uri("cerbo://guide/").is_none() && topic_of_uri("file:///etc/passwd").is_none());
    }

    #[test]
    fn the_skill_files_in_the_repository_are_current() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.agents/skills");
        for topic in &TOPICS {
            let path = root.join(format!("cerbo-mcp-{}/SKILL.md", topic.id));
            let want = topic.skill_file();
            if std::env::var_os("UPDATE_SKILL").is_some() {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &want).unwrap();
            }
            let have = std::fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(have, want, "run `UPDATE_SKILL=1 cargo test -p cerbo mcp_guide` to rewrite {}", path.display());
        }
    }
}
