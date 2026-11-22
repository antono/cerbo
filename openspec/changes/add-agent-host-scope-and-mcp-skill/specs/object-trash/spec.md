# Spec Delta

## ADDED Requirements

### Requirement: Restore from trash

A page that was moved to the trash SHALL be restorable by its UUID: the newest trash entry of that UUID SHALL be moved back unchanged, byte for byte, and the semantic index SHALL be updated. Restoring SHALL be refused, and nothing moved, when an object with that UUID is live or the trash holds no entry for it. Restoring SHALL NOT remove any other trash entry; emptying the trash stays the user's call.

#### Scenario: Restore
- **WHEN** a page was deleted and is restored by its UUID
- **THEN** its files SHALL be back at the object's place, identical to what was deleted, and the trash entry SHALL be gone

#### Scenario: Newest wins
- **WHEN** the trash holds two entries for one UUID
- **THEN** the newer SHALL be restored and the older SHALL stay in the trash

#### Scenario: Live object
- **WHEN** an object with the UUID exists
- **THEN** restoring SHALL fail with a message saying so and change nothing
