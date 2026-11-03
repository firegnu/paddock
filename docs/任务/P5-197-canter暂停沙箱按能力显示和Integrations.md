# 任务：canter 的暂停、沙箱按 corral 报的能力显示，停 canter 时提示没关掉的会话

2026-11-03，paddock/main 交给 paddock/dev-canter-caps（Claude Code，`opus[1m]` / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.64）、拿不准、拿不准；按钮状态和停止提示，做错一眼看得出、不碰数据，主控定不要交叉审查、改行为）
类型：功能变更
依据：canter/main 交来的需求单（`/tmp/canter-p2-193/paddock-需求单.md`）第 4、5 条，照 ranch R56 定下的公开约定做（ranch 主仓库 `docs/CONTRACT.md`「canter（R56）」一节，main `082d2a2`）。corral 的实现（R56b）还没部署，本轮只用假 corral 测。第 1～3 条在 P5-195，Instructions 在 P5-196，都已做完。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。

## 用户原话
- 需求单（canter/main 写，用户确认的范围）：
  - 第 4 条：「暂停：canter 不支持暂停，暂停按钮按 corral 报的能力禁用或不显示；现在 unsupported 会显示成「needs corral upgrade」，要改成如实说不支持。」「沙箱：fence 档案选择器对 canter 只列真正支持的（这一版 canter 不支持 fence 档案）；如果要显示 canter 自带的沙箱，写清楚它是「模型执行沙箱」，和 fence 的整个进程档案分开，不画成同一把锁。」
  - 第 5 条：「Integrations 页：多出的 canter 一行和它的状态来源说明照 corral 报的内容显示。」
- 用户 11-03：「也就是说这一版不做pause和fence，其他都会做，包括paddock相关的。」
- 用户 11-03 定 ranch 的方案：integration status 里 canter 是 `hooks:"launch"`（paddock 显示 Built in），不加状态来源字段。
- 主控问的两处（11-03 用户选）：暂停按钮「变灰，悬停说明」；停 canter 时没关掉的会话「底部提示一句」。

## 先读
- 仓库根的 `CLAUDE.md`。
- ranch 只读：`/Users/firegnu/Developer/personal_projs/ranch/docs/CONTRACT.md`「canter（R56）」一节。以它为准；下面摘的和它不一致时停下来问主控。不改 ranch。
- `docs/DESIGN.md` §13 P5-33（暂停）、P5-120（sandbox 界面）、P5-94（Integrations）、「接 canter（P5-195）」。

## corral 约定摘要（ranch R56）
- canter 的 `corral ls`、`corral status` 每个 agent，和 `corral integration status` 的 canter 一行，都带 `unsupported:["pause","sandbox"]`；别家没有这个字段（当作空）。
- `corral pause|resume` 对 canter：`ok:false`、`error:"unsupported_kind"`、退出码 1。老 corral 不会暂停的是 `unsupported`（退出码 9，要升级），两个分开。
- `corral start --sandbox <档案> -- canter`（或继承 child-sandbox）：`unsupported_kind`、退出码 1。`fence profiles` 的 kinds 不列 canter。canter 自带的沙箱 corral 不报字段，`status` 的 `sandbox` 对 canter 恒为 null。
- integration status 的 canter 一行：`hooks:"launch"`，skills target 是 codex，没有 install 段，排在 kimi 后。
- 状态可能是 starting／working／idle／blocked／exiting／unknown；卡在 canter 的信任框时是 starting。
- `corral stop` 对 canter：没确认关掉的会话多一个 `remaining_sessions` 数组，仍 `ok:true`；一次最多约 18 秒（paddock 停的超时是 120 秒，够）。
- `--prompt` 照旧；corral 自己加 `--session-file`、`--close-on-exit`，paddock 不传（New Agent 默认命令 `canter --yolo` 本来就没有）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-197-canter-caps`，分支 `p5-197-canter-caps`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-197-canter-caps`（已从 `.target/main` 克隆）。命令在 `app/` 下跑。
- 会动的大致是：`corral.rs`（Agent 读 `unsupported`）、`pause.rs`、`sidebar.rs`（卡片暂停按钮、暂停全部）、`window.rs`（停止后的底部提示）、`new_agent.rs`、`integrations.rs` 的测试、`docs/DESIGN.md` 和本文件。以实际需要为准。

## 要做的

### 一、暂停
1. `Agent` 读 `unsupported`（没有就是空）。
2. 列着 `pause` 的 agent，卡片上的暂停按钮变灰、点不动，悬停说明 `Pausing isn't supported for <kind>`（文字可微调，英文）。暂停中的老状态不受影响。
3. 「暂停全部」不算上不能暂停的：既不去暂停它，也不因为它而一直显示成“还有没暂停的”；只剩不能暂停的 agent 时，按钮照没有活 agent 处理。
4. `corral pause|resume` 报 `unsupported_kind` 时，底部写不支持（例如 `Couldn't pause <name>: not supported for canter`），不写 `needs corral upgrade`；`unsupported` 仍写 `needs corral upgrade`。

### 二、沙箱
5. New Agent 的 fence 档案选择器对 canter 不给档案：现在按 `fence profiles` 的 kinds 决定，canter 不在里面应已如此；加一条测试钉住（预设里带了档案的 canter 也不带 `--sandbox` 起）。
6. canter 自带的沙箱：corral 不报字段，不显示；卡片上的 fence 锁照 `sandbox` 字段，canter 恒为 null，不出锁。不做别的。

### 三、Integrations
7. 不改显示：canter 一行照 `hooks:"launch"` 显示 Built in，排在 kimi 后。用假 corral 数据加一条测试：带 `unsupported` 字段的 canter 一行读得出、是 Built in、没有装卸按钮，别的行不变。

### 四、停 canter
8. `corral stop` 的回答带非空 `remaining_sessions` 时，底部提示一句，例如 `Stopped <name>; 2 canter sessions may still be open`（文字可微调，英文）；不弹框，仍算停成功。没有这个字段或为空时照旧。

### 五、状态
9. `exiting`、`unknown` 两种状态在卡片、看板上不出错、不被当成在干活（看现有怎么处理不认识的状态，照着来；现有已经妥当就只加测试）。

### 六、文档
10. `docs/DESIGN.md` 的「接 canter（P5-195）」一条补上本轮：能力字段、暂停变灰、`unsupported_kind`、自带沙箱不显示、停止提示。

## 怎么算做完
- 需求单第 4、5 条：
  1. 暂停按钮按 corral 报的能力禁用（变灰、悬停说明），不支持时如实说不支持，不再写 `needs corral upgrade`。
  2. fence 档案选择器对 canter 不给档案；不把 canter 自带的沙箱画成 fence 的锁。
  3. Integrations 的 canter 一行照 corral 报的内容显示。
- 用户另选的：停 canter 有没关掉的会话时底部提示一句。
- 现有几家的暂停、沙箱、Integrations、停止一点不变。
- 验证只做这些：给新逻辑加针对性单元测试（读 `unsupported`、单个暂停变灰、暂停全部跳过、`unsupported_kind` 和 `unsupported` 的文字、canter 不带 `--sandbox`、Integrations 的 canter 一行、停止带 `remaining_sessions` 的提示、`exiting`／`unknown`），先写测试看到失败再实现；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 真实 canter 要等 ranch R56b 部署后由主控和用户、canter 主控、ranch 主控联调，你不做。

## 不要做
- 只用合成数据和假 `corral` 脚本测试；不开真实 canter 或任何 corral agent，不碰 `corral ls` 里已有的 agent（不 stop／send／keys／pause）。
- 不让 canter 能暂停、继续，不给 canter 套 fence 档案（用户：「这一版不做pause和fence」）。
- 不读 canter、corral 的状态目录；不改 canter、ranch、Saddle 仓库。
- 不加 Integrations 的状态来源说明（用户定不加）。
- 不按名字批量杀进程（`pkill -f paddock` 这类）；停自己起的进程用启动时记下的 PID。
- 截图、起测试窗口照 CLAUDE.md「桌面窗口测试」；不用脚本模拟按键或鼠标。
- 碰到要拿主意的事，停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等。主控的决定写进完成记录。
- 不合并到 main，不推送。只在本分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：
  - `corral.rs`：`Agent` 读 `unsupported`（没有就是空），加 `can_pause()`。远程机器的 agent 走同一个解析，也带上。
  - 暂停：`card.rs` 给卡片加 `pause_off`（列着 `pause`、没在暂停时是悬停说明 `Pausing isn't supported for <kind>`）；`sidebar.rs` 据此把暂停按钮画成灰色（0.4 透明）、不响应点击、悬停说明，暂停中的照旧显示 Resume。`pause.rs` 的「暂停全部」只算能暂停或正暂停着的活 agent，只剩不能暂停的时按没有活 agent 处理。`corral pause|resume` 报 `unsupported_kind` 时写 `not supported for <kind>`（`window.rs` 从左侧栏的列表查种类传进去；查不到写 `not supported`），`unsupported` 仍写 `needs corral upgrade`。
  - 停止：`new_agent::stop_with` 返回 `remaining_sessions`，非空时底部写 `Stopped <name>; N canter session(s) may still be open`，不弹框、不标红、仍算停成功。
  - 沙箱、Integrations、`exiting`／`unknown`：现有代码已经对（canter 不在 fence 的 kinds 里；`hooks:"launch"` 显示 Built in；不认识的状态显示 Unknown、不算在干活），只加测试。
  - `docs/DESIGN.md`「接 canter（P5-195）」补了本轮一条。
- 验证了什么：先写测试，读 `unsupported`、单个暂停变灰、暂停全部跳过、`unsupported_kind` 的文字、停止带 `remaining_sessions` 五条先看到断言失败再实现；canter 不带 `--sandbox`、Integrations 的 canter 一行、`exiting`／`unknown` 三条一写就过（现状已对）。`tests/pause.rs` 用假 corral 加一条 `unsupported_kind` 和老 corral `unsupported` 并列的测试。`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 全过。没起窗口、没截图，灰按钮和悬停说明的样子留给用户实际看。
- 拿主意的地方：
  - 公开的 `pause::run` 保持三个参数（集成测试在用），不带种类；只有 `run_with` 多一个查种类的参数，窗口用它。
  - 停止的提示不标红（任务说仍算停成功）；单数写 `1 canter session`。
  - 种类缺失时灰按钮写 `Pausing isn't supported for this agent`（canter 实际都带 `kind`）。
- 没做的事：真实 canter 联调等 R56b 部署。
- 踩过的坑：一开始改了公开的 `pause::run` 签名，`tests/pause.rs` 编不过，改回三个参数，只改 `run_with`。
- 主控的决定（11-03，第一轮回复后）：窗口菜单里的 Pause 项也要对不能暂停的 agent 变灰、点不动，和卡片按钮同一个判断，别家不变。已做：`sidebar::pause_item` 多一个 `can_pause` 参数（不能暂停、没在暂停时不可选，暂停中的仍是 Resume），窗口从左侧栏的列表读（`Sidebar::can_pause`，列表里没有的当能暂停）；加测试 `pause_is_off_for_an_agent_whose_kind_cannot_pause`，先看到失败再实现。只跑了这条和直接相关的测试（菜单项、卡片按钮、暂停全部、读 `unsupported`），加 `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，都过；没再跑全套。DESIGN 里原来写的“窗口菜单的 Pause 项不看能力”同步改了。
