# Pet packs

Taken from Saddle at commit `df1c727` (`assets/pets/`), image packs only: paddock draws pets as pixel pictures, so Saddle's block-glyph packs (`<name>.toml`), which were the fallback for terminals without images, are not used. The packs are compiled into paddock; nothing is read from disk at runtime. `src/pet.rs` parses them and its tests check every built-in pack.

A pet walks back and forth in the spare space right of the tabs, standing on the floor of the tab strip, and stops now and then to play one action.

## Format

A pack is TOML.

- `size = [width, height]`: the size of every pose, in pixels.
- `step_ticks`: ticks per step of travel while walking. Twelve ticks are one second. The walking clip must be a whole number of steps long, and should change pose on the same ticks, so the body and the feet move together.
- `mirror`: `true` flips every pose left to right when the pet heads left. Use it for a pet drawn facing right.
- `[palette]`: one letter per color, `"#rrggbb"`. `.` always means transparent.
- `[poses.<name>]`: `pixels`, `height` rows of `width` palette letters, one per pixel.
- `[[clip]]`: `name`, and `frames` as `[pose, ticks]` pairs.
  - `walking` loops while the pet moves.
  - `turning` plays at each end of the lane. The pet changes heading halfway through it.
  - `<name>-left`, when present, is used instead of `<name>` while the pet heads left, and must last as long. It takes the place of mirroring for that clip.
  - Every other clip is an action, picked at random between walks.

## The pets

From Saddle's `assets/pets/README.md`:

- **Clawd** (`clawd-image.toml`): converted frame by frame from the claude.dev references listed in `clawd-sources.json` (reference URLs, SHA-256 hashes and original frame counts). Original character and gesture references: Anthropic, <https://claude.dev/>, extracted 2026-10-01. The references are pixel art at 12 frames per second, like paddock, so each frame keeps its tick. The pack is the bottom 23 of the 37 pixel rows, 55x23 pixels, and holds the 20 actions that fit in that height. Every clip, actions included, is flipped when Clawd heads left. Clawd is Claude Code's mascot: fine for personal use; whether a distributed paddock ships it is undecided (`docs/DESIGN.md` §7).
- **Cat** (`cat-image.toml`): an original orange cat designed for Saddle, 55x23 pixels: a dark brown outline, a round head, big eyes with a white shine, a white muzzle, pink cheeks and inner ears, short legs. While it walks only the legs move.
- **Capybara** (`capybara-image.toml`): an original sleepy capybara designed for Saddle, in side view, 55x24 pixels with the cat's outline and colors; the extra row is headroom for the orange and the bird. It walks a step every 6 ticks, slower than the cat. Its actions: daze, munch a leaf, nap, balance an orange and let a bird land on its back.
