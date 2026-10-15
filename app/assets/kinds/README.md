# Agent kind icons

The sidebar draws one of these after an agent's name, tinted with the theme's colour for its kind.
They are compiled into the program (`src/kind_icon.rs`).

On a machine with its own originals in `~/.config/paddock/icons/` (`claude`, `codex`, `pi`, `omp`,
`amp`, `agy`; `.svg` or `.png`, the SVG when both are there), those are drawn instead, in their own
colours, with their transparent margins cut away; one that is missing or cannot be read falls back
to the silhouette here. That folder stays on the machine: its files are never added to this
repository.

| File | Source | Licence |
| --- | --- | --- |
| `pi.svg` | `https://pi.dev/favicon.svg`, downloaded 2026-10-06, unchanged. From the pi press kit (`https://pi.dev/press-kit`), intended for compact badges. | The pi site's footer says “MIT License”; the press kit states no other terms. |
| `omp.svg` | `assets/icon.svg` of `https://github.com/can1357/oh-my-pi` (file last changed in commit `2be3543`, `main` at `fc6c0c9`), downloaded 2026-10-06, unchanged. | MIT, Copyright (c) 2025 Mario Zechner, (c) 2025-2026 Can Bölük, (c) 2026 Stencil Labs, Inc.; the full notice is in `LICENSE-omp`. |
| `claude.svg` | Original drawing for paddock, not the official mark. | Part of paddock. |
| `codex.svg` | Original drawing for paddock, not the official mark. | Part of paddock. |
| `amp.svg` | Original drawing for paddock, not the official mark. | Part of paddock. |
| `agy.svg` | Original drawing for paddock, not the official mark. | Part of paddock. |

`claude.svg`, `codex.svg`, `amp.svg` and `agy.svg` are original drawings for paddock, not the
official marks; Claude and Codex are trademarks of Anthropic and OpenAI, Amp of its makers,
Antigravity of Google. They borrow only the kinds' colours and ideas (a spark, a terminal prompt,
an amplifier's level bars, a chevron lifting off the ground), not the shapes of any of their logos.

These icons must be reviewed again before paddock is distributed.
