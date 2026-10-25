# Durable Writes

## Purpose
Define the durability contract for every write Cerbo makes into a vault: an interrupted,
failed, or concurrent write SHALL never leave a vault file truncated, partially written, or
destroyed. This capability exists because `page.md` is frequently the only copy of a user's
note.

## ADDED Requirements

### Requirement: Atomic replacement of vault files
Every write to a file inside a vault SHALL be atomic with respect to readers: a reader
SHALL observe either the complete previous content or the complete new content, never a
partial or empty state. The system SHALL achieve this by writing to a temporary file in the
same directory as the destination, flushing its contents to storage, and then atomically
renaming it over the destination.

#### Scenario: Save interrupted by process termination
- **WHEN** a page save is interrupted by process termination partway through writing
- **THEN** the destination file SHALL still contain its complete previous content
- **THEN** no zero-length or truncated `page.md` SHALL exist in the vault

#### Scenario: Save fails because the filesystem is full
- **WHEN** a page whose content is 137 KB is saved onto a filesystem with 40 KB free
- **THEN** the write SHALL fail with an error reported to the caller
- **THEN** the destination file SHALL still contain its complete previous content
- **THEN** no partial content SHALL be observable at the destination path

#### Scenario: Concurrent readers never observe a partial write
- **WHEN** one process is writing a vault file and another reads the same path concurrently
- **THEN** the reader SHALL observe either the complete old content or the complete new content

#### Scenario: Temporary files do not leak into the vault view
- **WHEN** a write is interrupted before the rename completes
- **THEN** any leftover temporary file SHALL be excluded from page listings and link resolution
- **THEN** a subsequent successful write to the same destination SHALL succeed

### Requirement: Durability across power loss
A write reported as successful SHALL have had its contents flushed to persistent storage
before the rename, and the containing directory entry SHALL be flushed after the rename, so
that the new content survives an abrupt power loss.

#### Scenario: Power loss immediately after a reported-successful save
- **WHEN** a save returns success and power is lost immediately afterwards
- **THEN** after restart the file SHALL contain the newly saved content
- **THEN** the file SHALL NOT be empty or truncated

#### Scenario: Power loss during a save
- **WHEN** power is lost while a save is in progress and before it returns
- **THEN** after restart the file SHALL contain either the complete previous or the complete new content

### Requirement: Uniform application across all vault writes
Every component that writes vault files — page content, object metadata, backreferences,
annotations, and vault-level configuration and state — SHALL use the atomic replacement
protocol. No vault write path SHALL bypass it.

#### Scenario: Metadata write is atomic
- **WHEN** an object's metadata file is updated
- **THEN** the write SHALL use the atomic replacement protocol

#### Scenario: Backreference write is atomic
- **WHEN** an object's backreference file is rewritten
- **THEN** the write SHALL use the atomic replacement protocol

#### Scenario: A new non-atomic write path is rejected
- **WHEN** a code change introduces a direct, non-atomic write to a vault file
- **THEN** the project's automated checks SHALL fail

### Requirement: Write errors are reported, not swallowed
A failed vault write SHALL propagate an error to its caller. The system SHALL NOT report
success, and SHALL NOT silently discard the failure, when a vault file could not be written.

#### Scenario: Metadata update fails during a page save
- **WHEN** page content is written successfully but the metadata update fails
- **THEN** the save operation SHALL report an error identifying which part failed

#### Scenario: Backreference update fails during a page save
- **WHEN** page content is written successfully but a backreference update fails
- **THEN** the save operation SHALL report an error rather than returning success
