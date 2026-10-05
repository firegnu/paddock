# paddock 应用

一个 GPUI 窗口：左侧 Agents 侧栏（按组列出 corral agent，点击接入，底部可开新 shell），右侧带标题栏的终端窗格。原型阶段的实测记录、复用结果和已知不足见 [`docs/原型实测记录.md`](../docs/原型实测记录.md)。

## 构建

```sh
export CARGO_TARGET_DIR="$HOME/Developer/personal_projs/paddock-worktrees/.target"
M=app/Cargo.toml   # 在仓库根目录执行；其他目录写完整路径

cargo build --release --manifest-path "$M"
cargo test --manifest-path "$M" --all-targets
cargo clippy --manifest-path "$M" --all-targets -- -D warnings
```

独立的 `[workspace]` 和 `Cargo.lock`；Saddle 按固定提交号引用，`gpui-pre-*` 精确固定（见 `AGENTS.md`）。GPUI 用 `runtime_shaders` 在启动时编译着色器，不需要 Xcode 的 Metal 工具链。

## 运行

```sh
B="$CARGO_TARGET_DIR/release/paddock"
"$B" --cwd ~/some/dir                       # 交互 shell（$SHELL -i），标题栏显示 shell · <cwd>
"$B" --attach NAME                          # corral attach NAME，标题栏显示 agent 名字；关窗口只断开接入
"$B" -- /bin/sh -c 'cat app/samples/render.ans; sleep 600'   # 任意程序
"$B" --help
```

## 命令行选项

| 选项 | 说明 |
| --- | --- |
| `--attach NAME` | 接入已有的 corral agent |
| `--corral PROGRAM` | corral 命令（默认 `corral`） |
| `--cwd DIR` | shell 和 `--` 程序的工作目录（默认当前目录） |
| `--font FAMILY` | 终端字体（默认 Menlo） |
| `--fallback FAMILY` | 后备字体，可重复；默认依次尝试 Symbols Nerd Font Mono、FiraCode Nerd Font Mono、FiraCode Nerd Font |
| `--size PX` | 字号（默认 14） |
| `--line-height 倍数` | 行高（默认 1.3） |
| `--bounds X,Y,W,H` | 窗口位置和大小，单位 pt（默认 1000×640 居中） |
| `--stats` | 每秒向 stderr 打印帧数和耗时 |
| `-- PROGRAM ARG…` | 直接在 PTY 上运行一个程序 |

环境变量 `GPUI_TERM_WINDOW_ID=1`：启动后向 stderr 打印 `window-id: N`，用 `screencapture -x -o -l N out.png` 只截本窗口。

启动时会清掉从 corral、Saddle、Claude Code、Codex 会话继承的身份变量（列表见 `src/main.rs` 的 `INHERITED`），免得窗格里的程序误以为自己在那个会话里。

## 配置文件

`~/.config/paddock/config.toml`。文件不存在等于全部默认；写了不认识的键会报错并退出。改完重启生效。

```toml
theme = "tide"          # dune | tide | lagoon；不写等于 dune
sidebar_width = 240     # 侧栏宽度，单位 pt；默认 240

[colors]                # 可选：覆盖单个颜色，其余跟随主题
focus = "yellow"
terminal_blue = "#7aa2f7"
```

### 主题

三套预置主题沿用 Saddle：界面颜色取自所引用 Saddle 提交的 Dune、Tide、Lagoon。终端窗格的配色由 paddock 自带：

| 主题 | 终端 16 色的来源 |
| --- | --- |
| `dune` | Gruvbox dark |
| `tide` | Nord，亮色由常规色调亮而来 |
| `lagoon` | Everforest dark，亮色由常规色调亮而来 |

终端默认字色、底色跟随 Saddle 主题的 `text`、`bg`（Dune 的这两项在 Saddle 里是“跟随外层终端”，paddock 用 Dune 的 `agents_text`、`agents_bg`）。各主题中除黑色外的 15 个基本色在默认底色上的对比度都不低于 3:1。

Saddle 的 `terminal` 主题（全部跟随外层终端）在 paddock 里没有外层终端可跟，不提供，写了会报错。未知的主题名也报错。

### `[colors]`

最终颜色＝预置主题＋`[colors]` 里写的项。

- 可写的键：
  - Saddle 的 41 个界面颜色，键名与 Saddle 相同（`bg`、`text`、`border`、`muted`、`focus`、`agents_bg`、`agents_text` …，完整列表见 Saddle `src/theme.rs` 的 `named_mut`）。
  - 终端配色 20 个：

    | 键 | 含义 |
    | --- | --- |
    | `terminal_black` `terminal_red` `terminal_green` `terminal_yellow` `terminal_blue` `terminal_magenta` `terminal_cyan` `terminal_gray` | 0–7 号常规色 |
    | `terminal_dark_gray` `terminal_light_red` `terminal_light_green` `terminal_light_yellow` `terminal_light_blue` `terminal_light_magenta` `terminal_light_cyan` `terminal_white` | 8–15 号亮色 |
    | `terminal_text` `terminal_bg` | 终端默认字色、底色（不写时跟随 `text`、`bg`） |
    | `terminal_cursor` | 光标 |
    | `terminal_selection` | 选区底色（选中的字保持原色） |

- 值的写法与 Saddle 相同：`#RRGGBB`、ANSI 颜色名（`black` `red` … `dark_gray` `light_red` … `white`）、`default`。
  - 界面颜色写 ANSI 名，取终端配色里对应的那一色（含 `terminal_*` 的覆盖）；写 `default`，当文字用是终端默认字色，当底色用是终端默认底色。
  - `terminal_*` 写 ANSI 名，取对应那一色（含其他 `terminal_*` 的 `#RRGGBB` 覆盖）；写 `default` 等于不覆盖。
- 未知的键、写错的值都会报错并指出键名，程序不启动。
- 256 色里 16 号以后的颜色立方和灰阶、程序直接给的 RGB、程序用 OSC 4 改的调色板，不受主题影响。
