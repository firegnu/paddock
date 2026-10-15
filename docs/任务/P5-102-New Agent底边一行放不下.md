# 任务：New Agent 底边重排：去掉边框、分出主次，一行放下

2026-10-15，paddock/main 记。
依据：用户 10-15 发来 New Agent 截图（Claude、`opus[1m] · xhigh`，Controller 掉到第二行）：“在集成agy之前先修一下这个界面的问题。下面那一行按钮在模型名字比较长的情况下，controller那个按钮会被挤到最下面”；问清 Controller 是开关后：“我觉得能否重新设计一下，哪怕这个new agent下面变成两行也行啊”；看过甲（两行）、乙（带小标题）、丙（Controller 挪右上角）：“等一下，我总觉得这个设计还是不够优雅，主要是ui，不是逻辑。”主控提了一 去掉边框、二 分出主次、三 写成一句话，用户：“一和二合起来，出样稿吧”；看过丁1～丁3：“可以，照丁做吧”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-102-foot-row`，分支 `p5-102-foot-row`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-102-foot-row`。
- 动 `app/src/new_agent_view.rs` 的底边（`foot`）和本文件。

## 样稿
`docs/设计稿/P5-102-底边重排/`（原稿在 claude.ai 私有画布「New Agent底边重排样稿」）：丁1 Codex 普通 agent、丁2 Claude 主控、丁3 悬停。

## 要做的（照样稿丁）
- 底边去掉所有边框，排一行，不换行。
- 左边主块：agent 种类和模型·强度合进同一块淡底，中间一条细竖线分成两个点击区，各开各的选单（P5-99 的两个选单不变）；种类的字亮一档；强度字照旧随档位变色；有模型的种类才有右半。
- 主块旁边是项目：无框淡字，文件夹图标加名字。
- 右边：打开位置、Controller 只剩图标，一条细竖线，再是 `</>` 和开始按钮。
  - 打开位置的图标随选择变：This pane 整格填，New tab 顶上一个小标签，Split 往哪边就填哪半边。悬停提示 `Opens in <位置>`。
  - Controller 用皇冠图标：关着淡灰；开着强调色、带一层淡强调色底。悬停提示是开是关。点一下切换（行为不变）。
- 都是平时没底色，悬停出淡底；选单打开时它那块亮着。

## 怎么算做完
用户原话：“下面那一行按钮在模型名字比较长的情况下，controller那个按钮会被挤到最下面”；“照丁做吧”。

## 完成记录（10-15，主控）

### 做了什么
- `new_agent_view.rs`：底边照样稿丁重画。原来的描边胶囊（`chip`、`menu_chip`）换成无框的 `ghost`：平时没底色，悬停出淡底、字亮一档，选单开着时亮着。左边主块：种类（图标 18pt、字亮、稍粗）和模型·强度合在一块淡底里，中间一条细竖线，两半各开各的选单；种类没有模型时只有左半。项目是淡色文件夹图标加名字，名字太长截成“…”（只有它会缩）。右边：打开位置、Controller 只剩 32pt 的图标按钮，带悬停提示（`Opens in this pane`／`Opens in a new tab`／`Opens split right` 等，`Controller: on`／`off`）；一条细竖线后是 `</>` 和开始按钮。不再换行。
- 打开位置的图标随选择画：整格填、顶上小标签、分屏往哪边填哪半边；Controller 是皇冠，开时强调色、淡强调色底。打开位置的选单改成右边对齐挂出（图标在右边）。
- `docs/DESIGN.md` 记了这一条。

### 验证
- 纯界面改动，没造会红的测试。`cargo test --all-targets`（共 641 条）全过；`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都干净。
- 按丁的尺寸算：最长的 Codex 一组（`gpt-6.1-sol xhigh`、`Split right`）约 536pt，底边可用约 678pt，一行放得下。
- 没起窗口看：打开 New Agent 面板要按键，规矩不让脚本模拟，样子留给用户装上后看。

### 拿主意的地方
- 模型·强度中间的 `·` 照样稿去掉了，靠间距和强度的颜色分开。
- 提示文字用了现成的 `right_panel::Tip`。
- Controller 开着时悬停是更深一点的强调色底，不变回灰底。

### 没做
- 顶上预设那排、选单内容都没动。
