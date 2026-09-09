# Palette design notes

Preset JSON files are the source of truth. Imported palette mappings and attribution are recorded in each preset's `source` block and emitted into `NOTICES.md`.

## Harbor and Beacon

Harbor (light) and Beacon (dark) use a navy/apricot pairing based on a blue/orange relationship. Navy structures the writing and navigation surfaces; apricot identifies actions and selection. Keep the writing surface less saturated than the surrounding navigation so the two colors remain distinct without overwhelming the text.

## Shared constraints

- Keep search and comment highlights amber, review issues purple, and strengths green or teal.
- Ordinary and selected rows share foreground tokens, so selection backgrounds must keep those foregrounds readable.
- Editor selection is composited at 30% opacity; judge the resulting color over the writing surface.
