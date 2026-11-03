# Agent kind icons

The sidebar draws one of these after an agent's name, tinted with the theme's colour for its kind.
They are compiled into the program (`src/kind_icon.rs`).

On a machine with its own originals in `~/.config/paddock/icons/` (`claude`, `codex`, `pi`, `omp`,
`amp`, `agy`, `kimi`, `canter`; `.svg` or `.png`, the SVG when both are there), those are drawn instead, in their own
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
| `kimi.svg` | Original drawing for paddock, not the official mark. | Part of paddock. |
| `canter.svg` | Original drawing for paddock: a pony's head, the eye left open. | Part of paddock. |

`claude.svg`, `codex.svg`, `amp.svg`, `agy.svg` and `kimi.svg` are original drawings for paddock,
not the official marks; Claude and Codex are trademarks of Anthropic and OpenAI, Amp of its
makers, Antigravity of Google, Kimi of Moonshot AI. They borrow only the kinds' colours and ideas (a
spark, a terminal prompt, an amplifier's level bars, a chevron lifting off the ground, a crescent
moon and a star), not the shapes of any of their logos.

`canter.svg` is a pony's head drawn for paddock; canter, the user's own agent, has no logo, and the
only picture it has is a pixel pony in its welcome header.

These icons must be reviewed again before paddock is distributed.
