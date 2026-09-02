# demos

One recorded terminal demo per component, and the tapes that produce them.

## Why tapes and not screen recordings

A GIF made by pointing a recorder at someone's terminal is a screenshot of one
session: it drifts from the code silently, and nobody can tell whether it still
shows what the component does. A **tape** is a script — the keys, the timing,
the terminal size — so the GIF is a build artifact regenerated from the current
code, and a component whose demo no longer matches is a diff, not a vibe.

That is the same reason the acceptance corpus is data rather than prose.

```
just demos          # regenerate every GIF from its tape
just demo settings  # just one
```

Recorded with [VHS](https://github.com/charmbracelet/vhs).

## What each demo has to show

Not a feature tour — the BEHAVIOUR the acceptance properties pin, so the GIF
and the test are describing the same component:

| Demo | Shows |
|---|---|
| `settings` | ↑↓ through rows, ←→ dialling a value, a door that does not dial, Esc leaving without applying |
| `chooser` | picking from a list, the active item marked, an unlistable source explaining itself rather than looking broken |
| `sparkline` | a series at several widths, including narrower than its label |
| `butterfly` | two directions against a stable midline, at the width where the midline is all that fits |

A demo that only shows the happy path is advertising, not documentation.
