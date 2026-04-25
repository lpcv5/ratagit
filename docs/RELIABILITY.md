## Failure Handling

- Git errors must not crash UI
- Display errors in status panel

---

## Edge Cases

Must handle:

- empty repo
- detached HEAD
- merge conflicts
- large repos
- long filenames

---

## Performance

- avoid full redraw cost explosion
- track slow frames (>16ms)
