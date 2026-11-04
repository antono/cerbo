# Spec Delta

## MODIFIED Requirements

### Requirement: List pages
The system SHALL return a list of all pages by scanning `.cerbo/objects/` directories that contain `page.md`. Each entry SHALL include `uuid`, `title` and `virtualPath`, where `virtualPath` is the page's `cerbo:virtualPath` string or `null` when the predicate is absent or empty. The command SHALL NOT accept a `vaultId` parameter — page storage is global within the app context.

#### Scenario: page_list returns uuid entries
- **WHEN** the frontend calls `invoke('page_list')` without a `vaultId`
- **THEN** the system returns one entry per object with `page.md` and a valid page type
- **THEN** each entry includes `uuid` (UUID v4 string), `title` (from `meta.ttl`) and `virtualPath`
- **THEN** no entry includes a `slug` field

#### Scenario: page_list ignores vaultId parameter
- **WHEN** `page_list` is invoked with any extra parameter
- **THEN** the extra parameter is ignored
- **THEN** all pages in the configured objects directory are returned

#### Scenario: virtualPath values
- **WHEN** one page has `cerbo:virtualPath "notes/rust"` and another has no such predicate
- **THEN** the first entry has `virtualPath: "notes/rust"`
- **THEN** the second entry has `virtualPath: null`

#### Scenario: Invalid stored path is reported as null
- **WHEN** a page's `meta.ttl` holds a `cerbo:virtualPath` that fails validation
- **THEN** its entry has `virtualPath: null` and the page is listed at the root
- **THEN** the invalid value is left on disk for `cerbo index` to report
