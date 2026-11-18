use super::types::*;
use serde_json::json;

#[test]
fn content_blocks_round_trip() {
    let blocks = vec![
        ContentBlock::text("hi"),
        ContentBlock::resource_link("a", "cerbo://page/a"),
        ContentBlock::Image { data: "QQ==".into(), mime_type: "image/png".into(), uri: Some("file:///i.png".into()) },
        ContentBlock::Resource { resource: ResourceContents::Text { uri: "file:///a".into(), text: "t".into(), mime_type: Some("text/markdown".into()) } },
        ContentBlock::Resource { resource: ResourceContents::Blob { uri: "file:///b".into(), blob: "QQ==".into(), mime_type: None } },
    ];
    for b in blocks {
        assert_eq!(ContentBlock::from_value(&b.to_value()), b);
    }
    assert_eq!(ContentBlock::text("x").to_value(), json!({"type": "text", "text": "x"}));
}

#[test]
fn unknown_content_and_updates_are_kept() {
    let odd = json!({"type": "hologram", "x": 1});
    assert_eq!(ContentBlock::from_value(&odd), ContentBlock::Unknown(odd.clone()));
    // A known type with a missing required field is kept as unknown too.
    let bad = json!({"type": "text"});
    assert_eq!(ContentBlock::from_value(&bad), ContentBlock::Unknown(bad));

    let u = json!({"sessionUpdate": "notice", "severity": "info", "title": "t"});
    assert_eq!(SessionUpdate::from_value(&u), SessionUpdate::Other { kind: "notice".into(), raw: u.clone() });
    let broken = json!({"sessionUpdate": "tool_call"});
    assert!(matches!(SessionUpdate::from_value(&broken), SessionUpdate::Other { kind, .. } if kind == "tool_call"));
}

#[test]
fn unknown_enum_values_survive() {
    let u = json!({"sessionUpdate": "tool_call", "toolCallId": "1", "title": "t", "kind": "teleport", "status": "paused"});
    let SessionUpdate::ToolCall(c) = SessionUpdate::from_value(&u) else { panic!() };
    assert_eq!(c.kind, Some(ToolKind::Other("teleport".into())));
    assert_eq!(c.status.unwrap().as_str(), "paused");
}

#[test]
fn the_permission_outcome_is_nested() {
    assert_eq!(PermissionOutcome::Cancelled.to_result(), json!({"outcome": {"outcome": "cancelled"}}));
    assert_eq!(PermissionOutcome::Selected("o".into()).to_result(), json!({"outcome": {"outcome": "selected", "optionId": "o"}}));
}

#[test]
fn the_stdio_mcp_entry_has_no_type_and_env_pairs() {
    let s = McpServer::stdio("cerbo", "/bin/cerbo", vec!["mcp".into()], vec![("A".into(), "1".into())]);
    assert_eq!(s.to_value(), json!({"name": "cerbo", "command": "/bin/cerbo", "args": ["mcp"], "env": [{"name": "A", "value": "1"}]}));
}
