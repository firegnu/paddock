# paddock 应用

一个 GPUI 窗口：左侧 Agents 侧栏（按组列出 corral agent，点击接入，底部可开新 shell），右侧带标题栏的终端窗格。原型阶段的实测记录、复用结果和已知不足见 [`docs/原型实测记录.md`](../docs/原型实测记录.md)。

## 构建

```sh
export CARGO_TARGET_DIR="$HOME/Developer/personal_projs/paddock-worktrees/.target/main"  # worktree 里用分支名代替 main
M=app/Cargo.toml   # 在仓库根目录执行；其他目录写完整路径

cargo build --release --manifest-path "$M"
cargo test --manifest-path "$M" --all-targets
cargo clippy --manifest-path "$M" --all-targets -- -D warnings
```

独立的 `[workspace]` 和 `Cargo.lock`；不依赖 Saddle 的库，用到的 Saddle 代码已迁入 `src/`（文件开头注明来自 Saddle 哪个文件），`gpui-pre-*` 精确固定（见 `AGENTS.md`）。GPUI 用 `runtime_shaders` 在启动时编译着色器，不需要 Xcode 的 Metal 工具链。

## 运行

```sh
B="$CARGO_TARGET_DIR/release/paddock"
"$B" --cwd ~/some/dir                       # 交互 shell（$SHELL -i），标题栏显示 shell · <cwd>
"$B" --attach NAME                          # corral attach NAME，标题栏显示 agent 名字；关窗口只断开接入
"$B" -- /bin/sh -c 'cat app/samples/render.ans; sleep 600'   # 任意程序
"$B" --help
```

## 打包成 app

```sh
cargo build --release --manifest-path "$M"
cargo run --release --manifest-path "$M" --example bundle -- --install
```

在编译目录的 `release/` 下生成 `paddock.app`（`Info.plist`、程序、猫的像素图标，图标用系统自带的 `iconutil` 合成），`--install` 再复制到 `~/Applications/paddock.app`（只覆盖之前装的 paddock）。不签名、不公证。

从 Finder、Launchpad 或程序坞启动时，paddock 会向登录 shell 取一次 `PATH`（找得到 `corral`、`git` 和 shell 里的程序），新 shell 开在主目录。

## 设置页

菜单 paddock → Settings…（⌘,）打开单独的设置窗口（已开着就提到最前），分 General、Colors、Advanced 三页（照 Saddle 的 Settings）。改动先是草稿，Save（⌘S）才写入配置文件，Revert 放弃草稿；用红点或 ⌘W 关窗，有未保存的改动时先问 Save / Don't Save / Cancel（退出 paddock 时也会问）；每项可 Default 恢复默认。换主题会载入该主题的全部颜色并清掉颜色覆盖，之后改的颜色标 `custom`。保存只改动过的键，保留注释和其他内容；配置文件在别处被改过时不写，可选 Keep my edits 或 Discard my edits。主题、颜色、侧栏宽度、宠物保存后立即生效；字体、刷新间隔、corral 命令标 Restart required，重启后生效。

## 菜单与快捷键

| 菜单 | 项 |
| --- | --- |
| paddock | About paddock、Quit（⌘Q） |
| Shell | New Tab…（⌘T）、New Shell（⌘N）、Split Right…（⌘D）、Split Down…（⇧⌘D）、Split Left…、Split Up…、Close Pane（⌘W）、Close Tab（⇧⌘W） |
| Edit | Copy（⌘C，有选区时）、Paste（⌘V）、Find…（⌘F）、Find Next（⌘G）、Find Previous（⇧⌘G） |
| View | Fold Agents、Sort Agents by Name（勾选跟随侧栏）、Zoom Pane（⇧⌘↩） |
| Agent | New Agent…（⇧⌘N）、Stop Agent…、Go to Agent…（⌘P）、Attention…（⇧⌘A） |
| Window | Minimize（⌘M）、Zoom、Next Tab（⇧⌘]）、Previous Tab（⇧⌘[）、Tab 1–9（⌘1…⌘9，⌘9 是最后一个） |

新建 agent：菜单 Agent → New Agent…、侧栏底部 ＋ Agent，或 `+`、`Split ▾` 选择框里的 New agent…，打开单独的窗口：项目目录（可从列表选或 Choose… 用系统对话框）、Codex 或 Claude、Controller（名字固定 main）或 Regular、名字前缀（默认取目录名）、在哪里打开；Advanced 里可改完整命令、第一条消息，并显示将要执行的 `corral start …`。Create（⌘↩）成功后在选定位置接入；失败时窗口保留并显示原因。当前窗格是运行中的 shell 时不替换，改开新标签页。停止：Agent → Stop Agent… 或侧栏底部 ■ Stop，作用于当前窗格的 agent，系统提示框确认后调用 `corral stop`，结果显示在侧栏底部。

布局保存：标签页、分屏和每个窗格的内容（shell 及其目录、agent 名字）有变化就存进 `$XDG_STATE_HOME/paddock/layout.json`（没设时 `~/.local/state/paddock/layout.json`），退出时也存。下次不带 `--attach` 或 `-- 程序` 启动时恢复：shell 在原目录开新的（之前的命令不会重放），agent 按名字和实例号重新接入（不在了或换了实例会显示原因）。带 `--attach` 或 `-- 程序` 启动时不恢复，也不覆盖存档。存档读不出来时照常启动，侧栏底部说明原因，并且不覆盖那个文件。不读写 Saddle 的布局文件。

窗格放大：有多个窗格时，标题栏的 Zoom 或 View → Zoom Pane（⇧⌘↩）把当前窗格临时铺满终端区，Restore 或再按一次回到原来的分屏；其他窗格照常运行；切到别的窗格、关掉放大的窗格或再分屏时自动还原；按标签页分别记。

终端里查找（⌘F）：在当前窗格右上方打开查找栏，边输入边从视图底部往上找，选中并滚到找到的地方；回车或 ⌘G 找更早的一处，⇧回车或 ⇧⌘G 找更新的一处，两头绕回；按字面找，输入里有大写字母才区分大小写；找不到时显示 No match。Esc 或 × 关闭并回到最底部。

Go to Agent（⌘P）：在窗口上方打开搜索框，按名字或项目（工作目录最后一级，不分大小写）过滤 agent，也能搜到 Settings 的各页；↑↓ 选、回车或点击打开（agent 已在某个窗格就跳过去），Esc 或点外面收起。

Attention：侧栏头部的 `Attention · N`（有等待或出错时黄色，只有新回复时紫色）或 Agent → Attention…（⇧⌘A）打开浮动列表：等待中、出错的 agent 和 corral 读取失败在前，有新回复（一轮结束后还没在当前窗格看过）的在后；点一行或 ↑↓ 加回车打开那个 agent，Esc 或点外面收起。打开不会替 agent 回答。

关窗格、关标签页或退出时，会结束还活着的 shell 就先用系统提示框列出这些 shell，确认后才关；agent 窗格只是断开，agent 继续运行。新标签、分屏的选择框打开时 Esc 关闭。About paddock 是单独的小窗口，⌘W 关闭。其他按键都交给终端。

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

字体四项的优先级为：命令行 > 配置文件 > 上表的内置默认值。给了 `--fallback` 时，所有重复项组成的新列表整组替换配置里的 `font_fallbacks`，不合并。

启动时会清掉从 corral、Saddle、Claude Code、Codex 会话继承的身份变量（列表见 `src/main.rs` 的 `INHERITED`），免得窗格里的程序误以为自己在那个会话里。

## 配置文件

`~/.config/paddock/config.toml`。文件不存在等于全部默认；写了不认识的键会报错并退出。改完重启生效。

```toml
theme = "tide"          # dune | tide | lagoon；不写等于 dune
sidebar_width = 380     # 侧栏宽度，单位 pt；默认 380（Agents 面板信息较多，窄于 320 时“哪家”只显示图标、状态只显示动画）
font = "Geist Mono"
font_fallbacks = ["Sarasa Mono SC", "Maple Mono NF CN"]
font_size = 14.5
line_height = 1.3
mascot = "cat"          # 宠物：clawd | cat | capybara；默认 clawd
mascot_enabled = true   # 关掉宠物写 false；默认 true
refresh_ms = 1000       # 多久问一次 corral，毫秒；默认 1000
corral = "corral"       # corral 程序；命令行 --corral 可临时覆盖

[colors]                # 可选：覆盖单个颜色，其余跟随主题
focus = "yellow"
terminal_blue = "#7aa2f7"
```

字体配置的四个键都写在顶层（`[colors]` 之前），均可省略：

| 键 | 类型与说明 | 内置默认值 |
| --- | --- | --- |
| `font` | 字符串，终端字体 | `Menlo` |
| `font_fallbacks` | 字符串数组，按顺序尝试；`[]` 表示不设置后备字体 | `Symbols Nerd Font Mono`、`FiraCode Nerd Font Mono`、`FiraCode Nerd Font` |
| `font_size` | 数字，字号；支持整数或小数 | `14` |
| `line_height` | 数字，行高倍数；支持整数或小数 | `1.3` |

`font_size` 和 `line_height` 必须是有限正数。四项写错类型或数值不合法时，报错会指出键名；命令行指定的字号和行高也必须是有限正数。

### 宠物

标签条右侧的空地上有一只宠物来回走、时不时做个动作（照 Saddle 的宠物，只用图片版，见 `assets/pets/README.md`）。`mascot` 选 `clawd`、`cat` 或 `capybara`，写错会报错；`mascot_enabled = false` 关掉。空地放不下宠物时不显示。

### 主题

三套预置主题沿用 Saddle：界面颜色取自 Saddle 提交 `df1c727` 的 Dune、Tide、Lagoon，已复制进 `src/preset.rs` 独立维护。终端窗格的配色由 paddock 自带：

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
  - Saddle 的 41 个界面颜色，键名与 Saddle 相同（`bg`、`text`、`border`、`muted`、`focus`、`agents_bg`、`agents_text` …，完整列表见 `src/preset.rs` 的 `named_mut`）。
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
