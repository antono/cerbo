# Virtual Path Input

## REMOVED Requirements

### Requirement: Virtual path text input
**Reason**: This capability specifies a desktop dialog control — a text input with
autocomplete over existing virtual paths — that does not exist. The `cerbo:virtualPath`
predicate itself, its validation rules and its effect on the symlink tree are specified by
`uuid-object-storage` and `vault-symlink`, which is where they belong.

**Migration**: None. Callers supply `cerbo:virtualPath` when creating a page; see
`page-crud`.

### Requirement: Virtual path autocomplete
**Reason**: Same non-existent control.

**Migration**: None.

### Requirement: Virtual path validation
**Reason**: Duplicates the validation rules already stated normatively in
`uuid-object-storage`.

**Migration**: None. The rules are unchanged and are enforced when `meta.ttl` is read.

### Requirement: Path loading from vault context
**Reason**: Same non-existent control.

**Migration**: None.

### Requirement: Autocomplete selection via keyboard
**Reason**: Same non-existent control.

**Migration**: None.
