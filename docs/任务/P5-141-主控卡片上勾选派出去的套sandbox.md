# 任务：主控卡片上勾选，它之后派出去的 agent 套 sandbox

2026-10-19，paddock/main 记。R18 已在 ranch 合并、部署（本机 corral/ranch/fence 是 `versions/eff7a40`），ranch/main 经用户转告请 paddock 做这件。用户 10-19 看过样稿 E 和下面各条：“都同意，先做P5-142”。
依据：用户 10-19：“我的想法是在paddock中默认委派出去的不是sandbox中跑的，但是我勾选了某项之后就变成sandbox了，但是ranch中默认是不再sandbox中跑的。但是可以通过paddock选中的状态改为sandbox跑。我想要的是这个效果”；问“这个主控的勾选是启动主控之后的，还是中间随时可以变”，选了随时可改；用户不要改 `CLAUDE.md` 靠主控记规矩的做法（“还有更加优雅的办法吗？这个要改claude.md”），也不要 corral 反向读 paddock 的东西（“我不想corral反向关联paddock”）；最后：“选甲，写草稿吧”。
依赖：P5-142
执行：派出
（理由：和 P5-120 规模相近的界面活，改卡片、corral 解析、后台调用；主控审，再加 Codex 交叉审查。）

## 用户要的效果
- ranch 默认不变：主控派出去的 agent 不套墙。
- paddock 默认也不套；在 paddock 里勾上之后，这个主控**之后**派出去的 agent 套墙；随时能勾、能取消，从下一次派活起生效。
- 已经派出去、正在跑的 agent 不受影响。
- 主控自己不套墙。

## ranch 给的公开命令（R18，CONTRACT「corral child-sandbox」）
- `corral child-sandbox NAME`：查；`corral child-sandbox NAME <档案>`：设；`corral child-sandbox NAME --clear`：清。输出 `{ok, name, instance, child_sandbox: {"profile":"dev"} | null}`。
- 设档案时 corral 先跑 `fence check`，不过就报 `sandbox_failed`／`sandbox_unavailable`，旧设定不变；查和清不需要 fence。其他错误：`not_found`（退出码 2）、`usage`、`restarted`（这个名字换了实例）。
- 设定跟着这一次运行（instance）走，主控重启后就没了。
- `corral ls`／`status` 每个 agent 多一个 `child_sandbox` 字段（`null` 或 `{"profile":…}`）；老版本 corral 没有这个字段。
- 被设了的主控之后 `corral start`（corral 从 `CORRAL_NAME`／`CORRAL_INSTANCE` 认出调用者）就照 `--sandbox <档案>` 起；调用时显式写了 `--sandbox`／`--no-sandbox` 的以显式为准；读不了设定报 `child_sandbox_unreadable`、不起 agent。

## paddock 要做的
1. **读**：`corral.rs` 的 `Agent` 解析 `child_sandbox`（缺字段和 `null` 分开：缺字段 = 老 corral，不显示这个设置）。
2. **主控卡片展开详情多一格 `Delegates`**（样稿 E）：显示 `Not sandboxed` 或 `In sandbox · dev`，点开菜单选：`Not sandboxed`，加上 `fence profiles` 列出的档案（照 P5-120 New Agent 的写法：名字、来源、说明）。菜单底下一行说明：从下一次派活起生效，已在跑的不变，主控自己不进墙。
   - 选档案调 `corral child-sandbox NAME <档案>`，选不套调 `--clear`；都走 `command.rs`、后台跑，不卡界面。
   - 保存中这一格显示 `Saving…`；corral 拒绝时这一格变红，原样显示 corral 的错误码和说明，设定保持原样（以下一次 `corral ls` 为准）。
   - 主控重启后设定随之消失，这一格跟着 `corral ls` 变回 `Not sandboxed`，不另外提示。
3. **收起的主控卡片**：设了的在名字后面有一个小标记（箭头指向锁，蓝色，和 agent 自己套墙的灰锁区分），悬停说明 `Agents it starts run in sandbox: dev`。没设的什么都不显示，和现在一样。
4. **什么时候出现**：只在 `role=controller` 的 agent 上，只在本机（远端没有 fence）、找得到 fence、corral 输出里有 `child_sandbox` 字段时。
5. **仓库顶层加 `.fence.toml`**（ranch/main 建议，“这在你的任务里定”）：
   ```toml
   # 编译目录约定见 CLAUDE.md；主仓库和 worktree 共用此目录。
   write = ["${repo}/../paddock-worktrees/.target"]
   ```
   这样主控按设定派出去的 agent 不用再在 `corral start` 上带 `--env CARGO_TARGET_DIR` 就能写编译目录，派活规矩不用改。代价：墙里的 agent 能写所有分支的编译子目录（只是编译产物，不含源码）。

## 怎么算做完
- 用户原话：“我的想法是在paddock中默认委派出去的不是sandbox中跑的，但是我勾选了某项之后就变成sandbox了 … 我想要的是这个效果”；“中间随时可以变”。
- 主控补的（用户 10-19 同意）：
  1. 在 paddock 里把一个测试主控设成 `dev`，它 `corral start` 出去的 agent 在 `corral ls` 里 `sandbox` 是 `dev`；改回 `Not sandboxed` 后再派，`sandbox` 是 `null`；改之前已派出的那个不变。
  2. 加了 `.fence.toml` 后，`fence check --profile dev --kind claude --cwd <某个 worktree>` 的可写目录里有 `paddock-worktrees/.target`。
  3. 墙外三项检查（`cargo test --all-targets`、`clippy -D warnings`、`fmt --check`）照旧全过。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-141-child-sandbox`，分支 `p5-141-child-sandbox`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-141-child-sandbox`（已从 `.target/main` 克隆），每条 cargo 命令前都加。

## 规矩
- paddock 只调 `corral`、`fence` 公开命令；不读 corral、fence 的内部文件；不改 ranch。
- 不对用户的主控调 `corral child-sandbox` 设值（查可以）；实测用自己开的测试主控 `paddock/test-<名字>`（它得是 `--label role=controller` 才有这一格，所以这次不用 `role=test`，靠名字认），它派出去的也只用 `paddock/test-*`，用完都 `corral stop`。
- 界面文字英文。

## 派活补充（paddock/main，10-19 派出前写）
用户 10-19：“开始做P5-141(我理解这都是界面的活）”。是的：除了仓库顶层一个 `.fence.toml`，都是 paddock 界面和 `corral.rs` 的解析；ranch 不动。

### 先读
- 本文件全部；样稿 `docs/设计稿/P5-120-sandbox界面/Delegates.dc.html`（E）和同目录 `README.md` 最后一行。
- ranch `docs/CONTRACT.md`「`corral child-sandbox NAME [<档案> | --clear]`（R18）」一节（只读，路径 `../ranch/docs/CONTRACT.md`），以及 `ls`／`status` 里 `child_sandbox` 字段的说明。
- `docs/DESIGN.md` §13 P5-120 一条（卡片锁、Sandbox 一格、被拦记录浮窗的做法，本件照它的风格）、P5-74／P5-136（卡片展开详情）。
- 代码：`app/src/corral.rs`（`Agent`、`Sandbox`、`role()`）、`app/src/card.rs`（`Sandboxed`、展开详情的格子）、`app/src/sidebar.rs`（fence `Watcher`、卡片怎么拿到 fence 状态）、`app/src/sidebar_sandbox.rs`（P5-120 的浮窗）、`app/src/fence.rs`（`profiles`）、`app/src/new_agent_view.rs`（P5-120 的档案菜单写法）、`app/src/command.rs`（起子进程一律走这里，共用 `spawn_guard`）、`app/src/popover.rs`。

### 补充说明
- 菜单的档案列表照 P5-120 New Agent 的写法从 `fence profiles` 来；fence 没找到时这一格不出现。
- 调 `corral child-sandbox` 在后台跑，不卡界面；结果出来后马上刷新这一张卡（不用等下一轮 `corral ls`），但以 `corral ls` 为准。
- `corral` 输出里缺 `child_sandbox` 字段（老 corral）时不显示这一格；为 `null` 时显示 `Not sandboxed`。
- 在 `docs/DESIGN.md` §13 P5-120 那条后面加一条 P5-141，写定下的样子和行为（照本文件和样稿 E）。
- `.fence.toml` 加好后，用 `fence check --profile dev --kind claude --cwd <你的 worktree>`（只读的公开命令）确认可写目录里有 `paddock-worktrees/.target`，把相关几行写进完成记录。
- 测试用合成数据和假 `corral` 脚本（照 `sidebar.rs`、`new_agent_view.rs` 现有测试的做法），覆盖：解析三种情况（缺字段、`null`、有档案）、设和清各拼出什么命令、corral 报错时显示原文且设定不变、非 Controller 和远程不出现。

### 不要做
- 不改 ranch（包括 fence、corral 的代码和文档）；不读 `~/.fence/`、`~/.corral/` 下的文件，只调 `fence`、`corral` 公开命令。
- 不对 `corral ls` 里的任何 agent 调 `corral child-sandbox` 设值或清除（只读查询也不需要）；不 `corral start`、`stop`、`send`、`keys`，不 attach。真实链路的实测由主控做。
- 不起 paddock 窗口、不截图，不用脚本模拟按键或鼠标。
- 不改 New Agent、paddock ctl、看板等别处；不给非 Controller 的 agent 加这个设置。
- 不改 `gpui-pre-*` 依赖，不加依赖。不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不合并到 main，不推送。只在 `p5-141-child-sandbox` 上提交，提交说明用中文、首行以 `P5-141：` 开头。

### 做完
跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`（都在 `app/` 下，带上面的 `CARGO_TARGET_DIR`）。在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录
（2026-10-19，派出的 agent 记。）

### 做了什么
- `app/src/corral.rs`：`Agent` 多一个 `child_sandbox: Option<Option<Sandbox>>`，用自己的反序列化把缺字段（老 corral，`None`）和 `null`（`Some(None)`）分开；`Client::set_child_sandbox` 调 `corral child-sandbox NAME <档案>` 或 `--clear`（参数由 `child_sandbox_args` 拼），走 `command.rs`、30 秒超时；corral 拒绝时原样带回 `error`、`message`（`Refused`），没跑起来、没回 JSON 也给一句话。
- 新 `app/src/delegates.rs`（不画界面的部分）：`of()` 判断什么时候出现（`role=controller`、找得到 fence、corral 有这个字段）；`Delegates` 给格子的字（`Not sandboxed`／`In sandbox · dev`／`Saving…`）、收起卡片标记的悬停（`Agents it starts run in sandbox: dev`）、拒绝的原文（`错误码: 说明`）；`Saves` 按主控名字和 instance 记每次选择：保存中、被拒、刚存好。存好后立刻改这张卡；保存之前就开始的那轮 `corral ls` 仍按新值显示（免得闪回），之后开始的读取以 corral 为准；instance 变了（主控重启）就忘掉。
- 新 `app/src/sidebar_delegates.rs`（画界面）：展开卡片横跨两栏的 `Delegates` 格（锁、设定、下拉箭头；保存中写 `Saving…`、点不开；被拒时标签红、按钮淡红底、下面红字写 corral 原话，设定照旧）；菜单照样稿 E 和 New Agent 的写法（`Agents <名字> starts`、`Not sandboxed`／`As today.`、`fence profiles` 的档案和来源、说明、替换内置、读不了的原因，当前打勾，底下一行说明）；收起的主控卡片名字后面的蓝色“箭头指向锁”标记，宽度算进名字那一行的余量。每次点开菜单在后台重读 `fence profiles`（可取消，旧的回答不用）；选了以后在后台调 corral，回来后马上 `poller.refresh()` 再读一轮。
- `sidebar.rs`、`sidebar_workspaces.rs`、`card.rs`、`agents.rs`、`lib.rs`：接线（`Floating::Delegates`、`Card::delegates`、`Panel::delegates`、读完一轮 `corral ls` 后 `settle`）。
- 仓库顶层 `.fence.toml`（照本文件第 5 条原文）。
- `docs/DESIGN.md` §13 在 P5-120 后加 P5-141 一条。

### 验证了什么
- 新测试：`corral::tests` 三个（三种解析，和 `status` 合并后保留；设、清各拼出什么命令，用假 `corral` 脚本记下实际收到的参数：`child-sandbox p/main dev`、`child-sandbox p/main --clear`；拒绝时 `sandbox_failed` 和原话带回，乱回、找不到程序的情况）；`delegates::tests` 四个（只在本机 Controller 且有字段时出现，`regular`／`implementer`／`test`／无角色、没 fence（远程）、老 corral 都不出现；保存中→存好立刻显示、旧读取不闪回、新读取为准；拒绝显示原文且设定不变、下次选择清掉；重启后丢掉、corral 回答的 instance 不对不采用）；`card::tests::a_controller_card_carries_its_delegates_setting_once_fence_is_here`（整条 `lines()` 上只有 Controller 卡带设定，没 fence 时都不带）。
- `app/` 下 `cargo test --all-targets`：800 过 0 失败；`cargo clippy --all-targets -- -D warnings`：过；`cargo fmt --check`：过（都带 `CARGO_TARGET_DIR=…/.target/p5-141-child-sandbox`）。
- `fence check --profile dev --kind claude --cwd /Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-141-child-sandbox`：退出码 0；`layers` 里有 `{"path":".../p5-141-child-sandbox/.fence.toml","source":"repo"}`；`settings.filesystem.allowWrite` 里有 `"/Users/firegnu/Developer/personal_projs/paddock-worktrees/.target"`（`${repo}` 解析成主仓库，拼出来正好是共用编译目录）；`denyWrite` 里有这份 `.fence.toml` 自己。另外 `skipped` 里 dev 档案自带的 `${target}` 两条写着 “placeholder has no value (git branch or CARGO_TARGET_DIR not set)”，有了 `.fence.toml` 这条不影响写编译目录。
- 没有起 paddock 窗口、没有截图、没有碰任何真实 agent（按「不要做」）。验收第 1 条（真实主控设 `dev` 后派出的 agent `sandbox` 是 `dev`、改回后是 `null`、之前派的不变）要主控用测试主控实测，界面效果也留给用户看。

### 拿主意的地方
- 被拒的红字留到下一次选择或主控重启才消失（任务没说多久）；保存中不能再打开菜单，免得两次选择交错。
- 档案列表每次点开菜单重读，不放进左侧栏每 3 秒的 fence 后台线程（那条线只认版本和被拦记录）；读到之前菜单先显示 `Not sandboxed` 和 `Reading fence's profiles…`。
- 菜单标题和底下说明里用 agent 全名（如 `paddock/main`），不用卡片上可能被截短的短名。
- 选和现在一样的项只关菜单，不调 corral。
- “闪回”的处理：corral 接受的时刻之前开始的那轮读取，仍按刚保存的值显示；只对这一张卡、这一次运行，之后开始的读取完全以 `corral ls` 为准。
- 展开的卡片头部不画蓝色标记（样稿 E 里展开的 main 也没有），只在收起时画。
- 设了的档案后来从 fence 删了：按钮照 corral 写 `In sandbox · <名字>`，菜单里没有它可勾；没另外处理。

### 没做的事
- 真实链路实测（验收第 1 条）和界面实看：按派活补充留给主控／用户。
- 没给 `corral send` 现有的错误解析和新调用抽公共函数（不动现有代码）。

### 踩过的坑
- 一开始在 `Saves::answer` 里存好后调 `settle(agents, at)` 想顺手写回卡片，结果同一时刻 `at <= began` 会把刚存的记录当成“已被读取赶上”删掉；写完自查时发现（还没跑测试），改成直接写回那张卡。另外，测试是写完实现后补的，不是先红后绿。

## 主控实测真实链路（paddock/main，10-19，验收第 1 条）
自开测试主控 `paddock/test-ctl`（脚本，`--label role=controller`），用 `corral child-sandbox` 公开命令（界面点选留给用户）：设 `dev` 后它 `corral start` 出去的 `paddock/test-child-1` 是 `sandbox: {"profile":"dev"}`；`--clear` 后再派的 `paddock/test-child-2` 是 `sandbox: null`；`test-child-1` 仍是 `dev`；`test-ctl` 自己 `sandbox: null`。两个子 agent 已停；`test-ctl` 留到装好后给用户在它卡片上试界面。

## 返工 1（交叉审查，见主仓库 `docs/任务/P5-141-交叉审查.md`「审查意见」）
主控看过审查意见后定的：
1. **审查第 2 条（必须改）**：连续两次选择丢掉第一次成功的防闪回记录。照审查的改法：把最近已确认的值及时间和当前这次请求的 Busy／Refused 分开存；新请求开始或失败时，上一笔成功对更早开始的读取仍有保护，读到成功之后开始的读取再清掉。先补回归测试（审查里的顺序：未设 → 设 dev 成功 → 再选另一个 → 保存前的 `null` 读取到达 → 第二次被拒，应显示 `In sandbox · dev` 加拒绝原因；另外覆盖第二次成功、第二次清除），确认红，再改到绿。
2. **审查第 1 条（主控降为建议改，只做便宜的部分）**：菜单和选择绑定 `(名字, instance)`；`Floating::Delegates` 带上打开时的 instance，列表里这个名字的 instance 变了，菜单作废关掉；点选时这个名字当前列出的 instance 和菜单记的不一样就不调 corral。**不加** `corral status` 预检（仍不是原子的，主控不要）。corral 成功回答里的 instance 和请求的不一样时，照现在的做法丢掉保存状态即可（下一轮 `corral ls` 会照实显示）。补测试：instance 变了以后点选，假 corral 收到的写调用为零。
3. **审查第 4 条**：成功只认 `ok:true`，且必须有 `instance` 和 `child_sandbox` 两个字段（`child_sandbox` 显式 `null` 合法，缺字段不算清除）；否则按“corral's answer could not be read”拒绝、不改设定。补 `{}`、缺字段的假命令测试。
4. **审查第 5 条**：只在菜单确实打开时读档案；读的时候先清空旧列表，显示 `Reading fence's profiles…`；关菜单时取消还在跑的读取。
5. **审查第 6 条**：菜单照 New Agent 档案菜单的做法加最大高度和垂直滚动（不新加通用抽象）。
6. DESIGN 里 P5-141 那条“红字留到下一次选择”改成“下一次真正发给 corral 的选择”，并按上面 1、2、4、5 改写相应句子。

做完照原「做完」一节跑三项检查，在本文件末尾追加「## 返工 1 完成记录」（同样几样），提交首行 `P5-141：返工 1…`。

## 返工 1 完成记录
（2026-10-19，派出的 agent 记。）

### 做了什么
1. **连续两次选择丢掉防闪回记录（审查第 2 条）**：`delegates.rs` 的 `Saves` 每个主控一条 `Entry`：`confirmed`（corral 最近接受的值和时刻）和 `request`（这次的保存中／被拒）分开存。`begin` 只换 `request`，同一次运行的 `confirmed` 留着；`answer` 成功写 `confirmed`、清 `request`，失败只写 `request`；`settle` 用 `confirmed` 盖住在它之前开始的读取，读到它之后开始的读取才放下 `confirmed`，两样都没了才删这一条；实例变了整条丢掉。
2. **菜单绑定 (名字, instance)（审查第 1 条，只做便宜的部分）**：`Floating::Delegates { name, instance }`；格子只在实例对得上时画菜单；每次画之前（`tidy_delegates`）列表里这个名字换了实例或不在了，菜单关掉。点选的判断抽成 `delegates::pick`：这个名字列出的实例和菜单记的不一样、正在保存、或和现在一样，都返回 `None`，不调 corral；侧栏照它的结果在后台 `Pick::send`。没加 `corral status` 预检。corral 回答里的 instance 和请求的不一样，照旧丢掉保存状态。
3. **成功回答从严（审查第 4 条）**：`set_child_sandbox` 只在退出码 0 且 `ok` 为 `true` 时当成功，`ChildSandbox` 的 `instance`（现在是 `String`）和 `child_sandbox`（必须有，可以是 `null`）都是必填，不认识的字段照旧不管；退出码 0 但 `ok` 缺了或不是布尔，按 `corral's answer could not be read: it did not say ok` 拒绝；缺字段按 `corral's answer could not be read: <serde 的原因>` 拒绝；都不改设定。
4. **档案读取（审查第 5 条）**：只在这个主控的菜单确实刚打开时读 `fence profiles`，读前清空旧列表，菜单显示 `Reading fence's profiles…`；没有 Delegates 菜单开着时（`tidy_delegates`），还在跑的读取停掉（丢掉 `fence::Cancel` 就会让 fence 被停掉、回收）。
5. **菜单高度（审查第 6 条）**：`max_h` 取窗口高度减上下各 8，`overflow_y_scroll`，照 New Agent 档案菜单的做法，没加通用抽象。
6. **DESIGN**：P5-141 那条改成“红字留到下一次真正发给 corral 的选择或主控重启”，并补写了 1～5 对应的句子。

### 验证了什么
- 第 1 条先红后绿：按审查的顺序补 `a_second_choice_keeps_the_first_one_over_older_reads_when_refused`（未设 → 设 dev 成功 → 再选一个 → 保存前的 `null` 读取到达 → 第二次被拒，应显示 `In sandbox · dev` 加拒绝原因），改之前跑是红的（`left: Some(None)`，`right: Some(Some(Sandbox { profile: "dev" }))`），改后变绿。另外补的第二次成功、第二次清除两个方向（`a_second_choice_saved_stands_over_reads_older_than_it`、`a_second_choice_clearing_stands_over_reads_older_than_it`）在旧实现上本来就是绿的，留作覆盖。
- 第 3 条也先红：`only_a_whole_answer_counts_as_saved`（`{}`、没 `ok`、`ok:"yes"`、缺 `child_sandbox`、缺 `instance` 都应拒绝；完整回答带不认识的字段照常通过），改之前 `{}` 被当成 `Ok(ChildSandbox { instance: None, child_sandbox: None })`，改后绿。
- 第 2 条：`a_menu_for_a_run_gone_asks_corral_nothing` 用假 corral 记调用：菜单是 i-1 开的、列表已是 i-2 时点选，假 corral 一次也没被调（`calls` 文件不存在），名字不在了、选和现在一样的也不调；对得上的实例调一次（`child-sandbox p/main dev`），保存中再选不发第二次。
- `app/` 下 `cargo test --all-targets` 805 过 0 失败（比上次多这 5 个），`cargo clippy --all-targets -- -D warnings` 过，`cargo fmt --check` 过（带 `CARGO_TARGET_DIR=…/.target/p5-141-child-sandbox`）。
- 第 4、5 条（打开时读取、关掉时停、滚动）是界面接线，没有自动测试，也没起窗口，留给用户实看。

### 拿主意的地方
- 菜单作废的检查放在 `Sidebar::render` 开头（`tidy_delegates`），因为关菜单要 `window` 来还焦点，而轮询的 `poll` 拿不到 `window`；只有菜单真的过期时才调 `close_floating`，平时不做事。
- “确实打开”按这个名字的 Delegates 菜单在按下前没开、按下后开了来判断（直接从另一个主控的菜单点过来也会重读）。
- `pick` 只核对实例、是否正在保存、是否和现在一样，没再重查 Controller、fence、字段（主控说只做便宜的部分；格子本身已按这些条件才出现）。

### 没做的事
- 没加 `corral status` 预检（主控不要）；名字和实例之间的窗口仍在，要彻底挡住得在 ranch 加带期望实例的约定。
- 打开、关闭、滚动的界面效果没实看。

### 踩过的坑
- 用 perl 以 `|` 作分隔符做多行替换时，模式里转义的 `\|` 变成了正则的“或”，把 `Agent` 的 `pub error: Option<String>,` 里的 `error` 换掉了；编译前看 diff 发现，已修回（`git diff` 里 `Agent` 结构没有改动）。
