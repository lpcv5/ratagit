# INVARIANTS.md

These must ALWAYS hold:

---

## UI

- no overlapping text
- no out-of-bounds rendering
- focused panel is always visible

---

## State

- selection index is valid
- no dangling references
- no inconsistent git state

---

## Interaction

- key always maps to deterministic action
- no silent failures

---

## Git

- UI reflects real git state
- no stale data

---

## Testing

- all snapshots deterministic
- no flaky tests
