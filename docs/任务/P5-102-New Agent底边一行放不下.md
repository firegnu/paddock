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
