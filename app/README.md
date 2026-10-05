# paddock 应用

一个 GPUI 窗口：左侧 Agents 侧栏（目前是空白占位），右侧带标题栏的终端窗格。原型阶段的实测记录、复用结果和已知不足见 [`docs/原型实测记录.md`](../docs/原型实测记录.md)。

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
theme = "dune"          # 预置主题名（尚未生效，目前始终是 Dune）
sidebar_width = 240     # 侧栏宽度，单位 pt；默认 240

[colors]                # 覆盖单个颜色（尚未生效）
agents_bg = "#1d1a16"
```
