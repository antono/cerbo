# Smart Rename Cascade

## REMOVED Requirements

### Requirement: Index-driven link discovery
**Reason**: An optimisation of a cascade that no longer exists, expressed in terms of
`PageEntry` objects held in a link index — a structure the code never had and that this
change removes the last trace of. Renaming rewrites no other page, so there is nothing to
discover.

**Migration**: None. See the removal of the `rename-cascade` capability.
