# 任务：Lagoon 左侧栏和终端色温一致；窗口不在前台时磨砂不退成灰

2026-10-06，paddock/main 交给 paddock/dev-frost（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（常规 0.72，置信 0.57），按规则用常规；交叉审查不要；影响面拿不准（看得见 0.71），要改原生磨砂视图的状态，定为改行为）
类型：样式／文案调整
依据：本轮只调 Lagoon（必要时 Tide）的磨砂着色和磨砂视图在窗口不在前台时的外观；不改磨砂的区域、形状和别的界面。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、桌面窗口测试、`PADDOCK_NO_ACTIVATE`、截图不入库）。
- `docs/DESIGN.md` §13 里 P5-19（b～e，磨砂）和最后一条「P5-24 整体评审后的几处」。
- 样稿：`docs/设计稿/P5-24-窄条标签名色温/Lagoon.dc.html`（现状、A、B 三栏；磨砂只能近似）。
- 代码：`app/src/frost.rs`（`NSVisualEffectView` 怎么建、材质和混合方式）、`app/src/preset.rs` 的 `Preset::frost`（三套主题的 `wash`、`lit` 和注释）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-24b-frost`，分支 `p5-24b-frost`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-24b-frost`。
- 可以改：`app/src/frost.rs`、`app/src/preset.rs`（只改 `frost` 相关的值和注释）。
- 另有 `p5-24a-names` 分支在改 `sidebar.rs`、`window.rs`、`changes.rs`；你不要动这些文件。

## 要做的
用户原话：“6 我同意”（主控提的：Lagoon 下左侧栏是中性灰、终端是深青绿，冷暖不协调）；看过样稿：“我觉得挺好的。开工。”样稿里主控建议 A＋B。

1. **B：不在前台也保持激活外观**：现在磨砂视图跟着窗口是否在前台变化，不在前台时系统把它画成平的灰色。把它设成始终是激活的外观（`NSVisualEffectView` 的 state 设为 active 一类），先确认 AppKit 支持、在 macOS 27 上有效。做不到或有副作用（比如影响全屏、收起时的切换），停下来报告。
2. **A：Lagoon 着色加重**：把 Lagoon 的 `wash` 从 0.38 往上调（样稿约 0.6，具体值你在截图里对比后定），让左侧栏偏青绿、和终端同一色调，磨砂仍能透出一点桌面；P5-19c 的淡色字提亮要跟着检查，字仍然看得清。Tide 有同样的问题就一起调，Dune 不动；改了哪些写进完成记录。
3. 不改磨砂的区域和形状（P5-19e 定的：收起时标题栏整行不透明、窄条从标题栏下磨砂到底；展开时整列磨砂）。

## 怎么算做完
- 上面三条达到。
- 验证只做这些：`preset.rs` 里 frost 参数的测试照新值改好；在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`；用你编出来的 release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral（`ls` 给三四个 agent）起自己的窗口，截改前（main 上的程序或你改之前的构建）和改后的 Lagoon、Tide 各一张，左侧栏展开；窗口不在前台正好能看出 B 的效果。截图放 scratchpad，不入库，自己对比看过。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改磨砂区域、形状、别的界面；不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动，也不要让窗口抢到前台。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock。
- 不改系统设置，不用 sudo。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-24b-frost` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么（新的 wash 值）、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上改前改后截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **做了什么**：
  - B：`frost.rs` 建 `NSVisualEffectView` 时设 `setState(NSVisualEffectState::Active)`（AppKit 10.10 起就有，`objc2-app-kit` 现有的 `NSVisualEffectView` feature 已包含，`Cargo.toml`、`Cargo.lock` 不变）。窗口不在前台时磨砂不再退成平的灰色。
  - A：`preset.rs` 的 `Preset::frost`：Lagoon `wash` 0.38 → **0.72**，Tide `wash` 0.34 → **0.62**；Dune 不动；`lit`、`waiting`、`dim`、`dimmer` 都不动。Tide 的注释改写，说明为什么这两套洗得重。
- **验证了什么**：
  - `preset.rs` 里没有检查 frost 参数具体值的测试，没有可改的；`sidebar.rs` 的 `quiet_text_reads_on_the_frost_and_stays_in_full_screen` 按固定的灰底（#45494a）算对比度，与 `wash` 无关，照旧通过。
  - `app/` 下 `cargo test --all-targets` 全过（229 项）；`cargo clippy --all-targets -- -D warnings` 无警告（只有上游 `block v0.1.6` 的提示）；`cargo fmt --check` 通过。
  - 用 release 构建、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral（`ls` 给 4 个 agent）起自己的窗口，`screencapture -l` 截到了：改前（main 同一提交的构建）Lagoon、Tide 侧栏都是平的中性灰；只加 B 后侧栏出现透过来的桌面渐变（窗口不在前台，说明 Active 在 macOS 27 上有效）；再加 A，Lagoon 侧栏是深青绿灰，Tide 是蓝灰，和各自终端同一色调，下部仍能看出磨砂透出的渐变；淡色字（路径、分组名、时间）都看得清。每次按记下的 PID 停掉，没有残留进程。
- **拿主意的地方**：
  - 试过 Lagoon 0.60／Tide 0.52：截图里仍偏灰，色调只微微偏青／偏蓝，就加到 0.72／0.62。比样稿的约 0.6 重，是因为系统材质在 Active 时比样稿里假设的更亮、更灰，要更多着色才拉到终端的色调。
  - Tide 和 Lagoon 有同样的问题（中性灰配蓝灰终端），按任务一起调。
  - P5-19c 的提亮没改：洗得更重后底色更暗，淡色字只会更清楚，截图里看过。
- **没做的事**：
  - 没截窗口在前台时的样子（不能让窗口抢前台）；前台时本来就是 Active，B 不改变它，A 的观感前台后台一样。浅色桌面壁纸下透出来多少、收起时窄条、全屏切换的实际观感留给用户在真窗口看（B 只改材质外观，不碰区域和切换逻辑；全屏时磨砂本来就隐藏）。
  - 没改磨砂区域、形状、`sidebar.rs`、`window.rs`、DESIGN；没打包、没安装、没合并、没推送。
  - 过程中误在 `paddock-worktrees/` 下建了一个空文件 `p5-24b-frost-edit.tmp`，已移进 scratchpad，没有进提交。
