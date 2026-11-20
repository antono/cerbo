# Spec Delta

## ADDED Requirements

### Requirement: View tools for agents

`cerbo mcp` SHALL offer `view_list`, `view_get`, `view_create`, `view_update`
and `view_delete`, and `cerbo view list|show|create|set|delete` SHALL offer the
same operations on the CLI, with the same error strings. `view_create` and
`view_update` take the tree in the JSON shape of `view.toml`; `view_update`
replaces the whole tree atomically. `view_delete` moves the view to trash.

#### Scenario: Agent creates a view
- **WHEN** an agent calls `view_create` with a title and two valid page nodes
- **THEN** a view object is created and its UUID is returned
- **THEN** the desktop offers it in the dropdown on its next view list reload

#### Scenario: Invalid tree
- **WHEN** the tree names a UUID that is not a page
- **THEN** the call fails naming it and nothing is written

### Requirement: Permission levels

At permission `read`, the write tools (`view_create`, `view_update`,
`view_delete`) SHALL NOT be offered; `view_list` and `view_get` are. At
`approve`, each write tool call SHALL raise an approval card showing the
view title and the node count. At `auto` they run without a card.

#### Scenario: Read level
- **WHEN** the agent runs at `read`
- **THEN** its tool list contains `view_list` and `view_get` only

#### Scenario: Approve level
- **WHEN** an agent calls `view_create` at `approve`
- **THEN** nothing is written until the user approves the card

### Requirement: Agents cannot touch the default view or placement

No agent tool SHALL change the default view, any page's `virtualPath` or
`.cerbo/tree.toml` as a side effect of a view operation. The default view
SHALL be rejected as a target of `view_update` and `view_delete`.

#### Scenario: Default view target
- **WHEN** an agent calls `view_update` for the default view
- **THEN** the call fails and nothing changes

### Requirement: Skill text

The agent skill shipped with `cerbo mcp` SHALL describe views: when to make
one instead of moving pages, the node kinds, and that a page may appear in
many places.

#### Scenario: Skill matches tools
- **WHEN** the offered tools include the view tools
- **THEN** the skill text mentions them
