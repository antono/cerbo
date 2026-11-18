//! Mapping an agent's tool permission requests to the vault's agent
//! permission (design D3).
//!
//! The agent asks permission for its own tools with `session/request_permission`.
//! Under `read` and `approve` every such request becomes an approval card for
//! the person. Under `auto` only the agent's `cerbo` MCP tools, which reach the
//! vault through the session socket and so stay under the daemon's own checks,
//! are allowed without asking; anything else (a shell, the network, a tool this
//! code does not recognise) still asks. Ambiguity always asks.
//!
//! What goes back to the agent is `allow_once` or `reject_once`, never an
//! `always` option: a permission is decided again each time.

use crate::acp::{PermissionKind, PermissionOption, ToolCallUpdate, ToolKind};
use cerbo_core::agent::AgentPermission;
use cerbo_core::rpc::ApprovalOutcome;

/// What to do with a tool permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Allow it without asking a person.
    Allow,
    /// Show a card and wait for a person.
    Ask,
}

/// The decision for `tool` under the vault's `mode`.
pub fn decide(mode: AgentPermission, tool: &ToolCallUpdate) -> Decision {
    if mode == AgentPermission::Auto && is_vault_tool(tool) {
        Decision::Allow
    } else {
        Decision::Ask
    }
}

/// Whether the tool is one of the `cerbo` MCP tools of this session.
///
/// Conservative on purpose: the kind must be `read`, `search`, `edit`,
/// `delete` or `move` (`execute`, `fetch` and `other` never count), and the
/// tool's name, or the first word of its title, must be exactly
/// `cerbo_<tool>` (OpenCode) or `mcp__cerbo__<tool>` (Claude Code). A title
/// that merely mentions such a name, as a shell command line would, does not
/// count.
///
/// The name is all a permission request shows: ACP does not say which server a
/// tool belongs to, so a second MCP server that called itself `cerbo` could not
/// be told apart here. That is closed at the source instead of by matching: the
/// Claude session is started with `strictMcpConfig` (only the server the daemon
/// passes exists; a vault's `.mcp.json`, settings and plugins add none, see
/// `sandbox::claude_session_meta`) and OpenCode with
/// `OPENCODE_DISABLE_PROJECT_CONFIG` (a vault's `opencode.json` and `.opencode/`
/// are not read), and neither agent's home holds an MCP server of its own.
pub fn is_vault_tool(tool: &ToolCallUpdate) -> bool {
    let kind_ok = matches!(
        tool.kind,
        Some(ToolKind::Read | ToolKind::Search | ToolKind::Edit | ToolKind::Delete | ToolKind::Move)
    );
    if !kind_ok {
        return false;
    }
    let first_word = tool.title.as_deref().and_then(|t| t.split_whitespace().next());
    let named = [tool.name.as_deref(), first_word].into_iter().flatten().any(is_cerbo_tool_name);
    if !named {
        return false;
    }
    // A diff or a terminal in the card means the agent is changing a file or
    // running a command itself, whatever it calls the tool.
    let own_work = tool.content.as_ref().is_some_and(|content| {
        content.iter().any(|c| !matches!(c, crate::acp::ToolCallContent::Content { .. }))
    });
    !own_work
}

/// `cerbo_<tool>` or `mcp__cerbo__<tool>`, the tool being `[a-z0-9_]+`.
pub fn is_cerbo_tool_name(name: &str) -> bool {
    let rest = name.strip_prefix("cerbo_").or_else(|| name.strip_prefix("mcp__cerbo__"));
    rest.is_some_and(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
}

/// The word shown in a card for a tool kind.
pub fn kind_word(kind: Option<&ToolKind>) -> String {
    kind.map_or("other", ToolKind::as_str).to_string()
}

/// The option to answer with for a person's (or the clock's) decision: the id
/// of the first `allow_once` option for an approval, of the first
/// `reject_once` for a rejection or a timeout. `None` means answer
/// `cancelled`: no suitable option is offered, or the session ended.
/// An `always` option is never chosen.
pub fn option_for(outcome: ApprovalOutcome, options: &[PermissionOption]) -> Option<String> {
    let wanted = match outcome {
        ApprovalOutcome::Approved => PermissionKind::AllowOnce,
        ApprovalOutcome::Rejected | ApprovalOutcome::TimedOut => PermissionKind::RejectOnce,
        ApprovalOutcome::Cancelled => return None,
    };
    options.iter().find(|o| o.kind == wanted).map(|o| o.option_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(kind: ToolKind, title: &str) -> ToolCallUpdate {
        ToolCallUpdate { tool_call_id: "t".into(), title: Some(title.into()), kind: Some(kind), ..Default::default() }
    }

    fn opt(id: &str, kind: PermissionKind) -> PermissionOption {
        PermissionOption { option_id: id.into(), name: id.into(), kind }
    }

    #[test]
    fn auto_allows_only_the_cerbo_tools() {
        assert_eq!(decide(AgentPermission::Auto, &tool(ToolKind::Edit, "cerbo_edit_page")), Decision::Allow);
        assert_eq!(decide(AgentPermission::Auto, &tool(ToolKind::Read, "cerbo_read_page {\"page\":\"x\"}")), Decision::Allow);
        assert_eq!(decide(AgentPermission::Auto, &tool(ToolKind::Search, "mcp__cerbo__search_text")), Decision::Allow);
        let mut named = tool(ToolKind::Edit, "Edit page");
        named.name = Some("cerbo_edit_page".into());
        assert_eq!(decide(AgentPermission::Auto, &named), Decision::Allow);
    }

    #[test]
    fn everything_else_asks_even_under_auto() {
        for t in [
            tool(ToolKind::Execute, "cerbo_edit_page"),
            tool(ToolKind::Execute, "rm -rf cerbo_x"),
            tool(ToolKind::Fetch, "cerbo_fetch"),
            tool(ToolKind::OtherKind, "cerbo_x"),
            tool(ToolKind::Read, "Read /home/u/cerbo_notes.md"),
            tool(ToolKind::Read, "cat cerbo_x"),
            tool(ToolKind::Edit, "cerbo_"),
            tool(ToolKind::Edit, "cerbo_Edit"),
            tool(ToolKind::Edit, "my_cerbo_edit_page"),
            tool(ToolKind::Edit, "mcp__other__cerbo_x"),
            ToolCallUpdate { tool_call_id: "t".into(), ..Default::default() },
        ] {
            assert_eq!(decide(AgentPermission::Auto, &t), Decision::Ask, "{t:?}");
        }
        let mut with_diff = tool(ToolKind::Edit, "cerbo_edit_page");
        with_diff.content = Some(vec![crate::acp::ToolCallContent::Diff { path: "a".into(), old_text: None, new_text: "b".into() }]);
        assert_eq!(decide(AgentPermission::Auto, &with_diff), Decision::Ask);
    }

    #[test]
    fn read_and_approve_always_ask() {
        let t = tool(ToolKind::Read, "cerbo_read_page");
        assert_eq!(decide(AgentPermission::Read, &t), Decision::Ask);
        assert_eq!(decide(AgentPermission::Approve, &t), Decision::Ask);
    }

    #[test]
    fn answers_map_to_once_options_and_never_to_always() {
        let options = [
            opt("always", PermissionKind::AllowAlways),
            opt("once", PermissionKind::AllowOnce),
            opt("no-always", PermissionKind::RejectAlways),
            opt("no", PermissionKind::RejectOnce),
        ];
        assert_eq!(option_for(ApprovalOutcome::Approved, &options).as_deref(), Some("once"));
        assert_eq!(option_for(ApprovalOutcome::Rejected, &options).as_deref(), Some("no"));
        assert_eq!(option_for(ApprovalOutcome::TimedOut, &options).as_deref(), Some("no"));
        assert_eq!(option_for(ApprovalOutcome::Cancelled, &options), None);
        // Only `always` offered: nothing is approved with it; cancel instead.
        let only_always = [opt("always", PermissionKind::AllowAlways), opt("never", PermissionKind::RejectAlways)];
        assert_eq!(option_for(ApprovalOutcome::Approved, &only_always), None);
        assert_eq!(option_for(ApprovalOutcome::Rejected, &only_always), None);
    }
}
