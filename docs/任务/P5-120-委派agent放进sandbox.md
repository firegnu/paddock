# 任务：sandbox 在 paddock 里看得见、能选（界面）

依据：用户 10-16：“再加一个draft task。委派的agent放到sandbox中，要设计一套sandbox”。10-17 讨论后定下墙本身放在 ranch（corral 开 agent 时可选套上），讨论全文和已定的事在 ranch 的 `docs/任务/R17-agent放进sandbox.md`；用户：“对，搬到ranch。这边只留界面的东西”。10-19 主控出样稿 `docs/设计稿/P5-120-sandbox界面/`，用户：“其他我都同意”“我的理解是不要动ranch的代码。其他都同意”。
执行：派出
（理由：要动 New Agent、卡片、新浮窗、Settings 新页、预设存盘和一个新的 fence 调用模块，面广、量大，值得另一个 agent 做、主控审查再加交叉审查；主控对 fence 约定和样稿已经掌握，审查时能对上。）
你是被委派的 agent：照本文件做，不要再开别的 agent。

## ranch 那边已经好了的（ranch/main 10-19 补进本文件，ranch 的代码本件不动）
- ranch 的 R17b 已合并，10-19 部署到本机（`versions/7fbb76d`）：墙叫 **fence**，是单独的命令 `~/.local/bin/fence`，不挂在 `ranch` 下面。本机四家（Claude Code、Codex、pi、omp）关在墙里实测能照常干活；amp、agy 没有数据，带 `--sandbox` 报 `unsupported_kind`。
- 公开约定全文：ranch 仓库 `docs/CONTRACT.md`「sandbox 与 `fence`（R17b）」一节（只读），接的时候以那份为准。对照：

| 界面 | 用哪条命令 |
|---|---|
| New Agent 里选档案 | `fence profiles` 列档案和支持的种类（`kinds`）；`fence check --profile <档案> --kind <种类> --cwd <目录>` 看这一家在这个目录能不能用；`corral start NAME --sandbox <档案> [--label …] [--cwd …] -- <命令>` 起（`--sandbox` 在 `--` 前面，`--` 后面的命令照原样） |
| 卡片小锁、悬停写档案名 | `corral ls`、`corral status` 里每个 agent 的 `sandbox`：`{"profile":"dev"}`，没套墙或旧记录是 `null` |
| 被拦记录 | `fence denials --agent <名字> --instance <实例> --limit <n>`（实例取 `corral ls` 的 `instance`） |
| Settings 里看档案 | `fence profiles`、`fence check` 给出合并后的规则 |

- 找 fence：本机按 PATH 找 `fence`，看 `fence --version` 的 `contract` 是不是 `"1"`；没有或不是 `"1"` 就当没装，不提供 sandbox 选项（CONTRACT 原文）。远程机器（5090）没装 fence，本件远程一律不显示 sandbox。
- fence 前端命令只输出一行 JSON：成功 `ok:true`、退出码 0；失败 `ok:false`、`error`、`message`、退出码 1。忽略未知字段、显示未知取值。

### ranch/main 10-19 答 paddock/main 的六问（实测过）
1. 墙里 corral 所有用户命令都报 `sandbox`、退出码 6，主控套墙就派不了活；目前没有放开的计划。所以 Controller 开着时不让选 sandbox。
2. 跑着、一条拦截都没有：`ok:true`、`denials:[]`。`not_found` 只表示还没有这次运行的记录；`corral start` 刚返回的几十毫秒里也会是 `not_found`，当“还没有数据”，不当出错。
3. 拿不到“只要条数”：要数就带大的 `--limit`（如 5000）拉全量再数。一次一分钟的 Claude 会话 445 条、66 KB，一次调用不到 10 ms，每个套墙 agent 几秒查一次可以接受。但大多是杂音（那 445 条里 `other` 242、`file-read` 156、`file-write` 33、`network` 14），所以计数只数 `network` 和 `file-write`。
4. 检查过了、启动时墙才失败（少见）：`corral start` 照样返回成功，agent 一秒内退出，下一次 `corral ls` 后就没了；`corral read` 读不到原因（CONTRACT 里“可用 `corral read` 读取”与实际不符，paddock 不依赖这句）。能拿到的只有 `fence denials` 的 `exit:{"code":125,…}`，没有原因文字。用户定 ranch 不改，本件也不去追这种情况（见「不要做」）。
5. `fence check` 一次约 40–50 ms，只读。换种类时直接调；输入目录时加 200–300 ms 防抖。
6. 开 agent 只需在现有命令上加 `--sandbox <档案>`，别的不用改；只有 claude、codex、pi、omp 能套墙。

### 限制，界面照实显示
- **Settings 只能看、不能改档案**：fence 没有改档案的命令，paddock 也不能自己写 `~/.fence/` 里的文件。
- **文件拦截可能漏记**：网络拦截记录可靠；文件拦截来自系统日志，会漏条（R17b 实测 Codex、pi 写主仓库被拦没进记录，只在 agent 终端里看得到）。浮窗上标一句。
- 档案 `dev` 的说明和 `fence check` 的 `warnings` 照原文显示（如 Codex 的 `weaker_network_isolation`）。

## 先读
- 样稿 `docs/设计稿/P5-120-sandbox界面/README.md` 和其中各 `.dc.html`（C2 `DenialsTab` 未采用，不做）。
- ranch `docs/CONTRACT.md`「sandbox 与 `fence`（R17b）」一节（只读，路径 `../ranch/docs/CONTRACT.md`）。
- `docs/DESIGN.md`：P5-43、P5-99、P5-102（New Agent 浮层和底边）、P5-37/P5-135（预设）、P5-136、P5-74（卡片详情）、P5-44、P5-119（Settings 页）、P5-124（远程 New Agent）。
- 代码：`app/src/new_agent.rs`（`Preset`、`save_presets`、拼 `corral start` 命令）、`app/src/new_agent_view.rs`（底边 `foot()`、菜单 `menu()`、`command_block()`）、`app/src/corral.rs`（`Agent` 解析）、`app/src/card.rs`、`app/src/popover.rs`、`app/src/settings.rs`／`settings_view.rs`（`Page`）、`app/src/command.rs`（起子进程一律走这里，共用 `spawn_guard`）。

## 要做的（照样稿）
1. **fence 调用**：新模块按上面的约定调 `fence --version`、`profiles`、`check`、`denials`，解析 JSON（未知字段忽略），不在界面线程上跑。没装或 contract 不是 `"1"` 时，整个 sandbox 界面都不出现。
2. **New Agent（样稿 A、A2）**：
   - 底边皇冠左边加锁：默认 None（淡色开锁，悬停 `No sandbox`）；选了档案变亮写档案名，点开菜单列 None 和 `fence profiles` 的每个档案（名字、`built in`/用户、说明原文、读不了的档案照 `error` 标出），底下一行检查结果和 `See the rules`（打开 Settings › Sandbox）。
   - 选了档案时，对当前种类和目录跑 `fence check`（种类变立即跑，目录变防抖 200–300 ms）：通过且有 `warnings` 时锁带黄点、底边下照原文显示警告；不通过时锁变红、显示错误码和 fence 的 `message` 原文，Start（⌘↩）不能点。
   - Controller 开着时锁变灰不能点，悬停 `Controllers can't run in a sandbox: corral is blocked inside it`；打开 Controller 时已选的档案退回 None。
   - 种类不在 `fence profiles` 的 `kinds` 里（amp、agy、自定义）时退回 None、锁变灰。
   - 起 agent 时在 `corral start` 的 `--` 前加 `--sandbox <档案>`；`Will run` 里跟着显示。corral 回 `sandbox_unavailable`、`sandbox_failed` 时照现在起失败的做法把原文显示出来。
   - 远程机器的 New Agent 不出现锁。
3. **预设记住 sandbox**：预设多一个可选的档案字段，存进配置的 `agent_presets`；没写就是 None，老配置照旧能读。改了锁算改了预设（标 edited，可 Update preset）。Controller 角色的预设不存档案。应用预设时档案已不存在或这一种类不支持，就退回 None。
4. **卡片（样稿 B）**：`sandbox` 不是 `null` 的 agent，名字和强度后面一把小锁，悬停 `Sandbox: <档案>`，有计数时 `Sandbox: <档案> · N blocked`；N = `network` + `file-write` 条数，大于 0 时锁旁边显示黄色数字。展开后多一格 Sandbox：`<档案> · N blocked ›`，没有时 `<档案> · nothing blocked`，点了打开浮窗。计数对每个套墙的 agent 隔几秒取一次，`not_found` 当还没有数据。
5. **被拦记录浮窗（样稿 C1）**：从卡片弹出；标题 `Blocked`、agent 名和档案、关闭；`Network & writes`（默认，数字和卡片一致）／`All` 切换；每行本地时间、类型签（Network／Write／Read／Other，不认识的类型照原值）、目标（`~` 缩短，悬停看全路径；`target:null` 时显示 `detail`）、`detail` 原文；新的在上；空时 `Nothing blocked yet.`；`truncated:true` 时最后一行说明 fence 已停止记录；开着时跟着刷新；底下一句 `Network blocks are all here. File blocks may be missing; the agent's terminal shows every one.`。
6. **Settings › Sandbox（样稿 D）**：新的一页，放在 Instructions 和 Diagnostics 之间；只看：fence 版本状态行、档案列表（名字、来源、说明、`overrides_builtin`、`error`）、本机补充文件的路径和是否存在；选一个档案、一家种类（`kinds` 里的）和一个目录（默认最近的项目）后，显示 `fence check` 的来源层、允许的域名、能读、能写、不能读、不能写、跳过项、警告（照原文）。不提供任何改档案的入口，页上写明 `View only. Profiles are changed in fence, not here.`。没装 fence 时这一页一句话说明。
7. `docs/DESIGN.md` §13 补一条 P5-120，写定下的做法和理由。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-120-sandbox-ui`，分支 `p5-120-sandbox-ui`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-120-sandbox-ui`（已从 `.target/main` 克隆），cargo 命令在 `app/` 下跑。
- 测试用合成的 JSON 和假 `fence`、假 `corral` 脚本，不调真实的 fence 去起东西；只读的 `fence --version`、`profiles`、`check` 可以拿本机真的对照输出格式。

## 怎么算做完
- 用户原话：“这边只留界面的东西”；看过样稿：“其他我都同意”“我的理解是不要动ranch的代码。其他都同意”。
- 上面「要做的」1–7 照样稿做到。
- 验证：`git diff --check`；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。界面的样子、点选和键盘留给用户装好后实际看。

## 不要做
- 不改 ranch（包括 fence、corral 的代码和文档）；不读 `~/.fence/` 下的文件、corral 的 `meta.json`，只调 `fence`、`corral` 公开命令。
- 不做 C2 右侧栏标签；不做改档案；不追“启动后墙才失败（退出码 125）”的提示；远程不显示 sandbox。
- 不改 paddock ctl、看板等别处开 agent 的路子，只改 New Agent。
- 不起 paddock 窗口、不截图，不用脚本模拟按键或鼠标；不碰 `corral ls` 里的任何 agent，不用 `--sandbox` 真起 agent。
- 不改 `gpui-pre-*` 依赖，不加依赖。不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不合并到 main，不推送。只在 `p5-120-sandbox-ui` 上提交，提交说明用中文、首行以 `P5-120：` 开头。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么
1. **fence 模块**（新 `app/src/fence.rs`）：`version`（只认 `contract` `"1"`）、`profiles`、`check`、`denials`，一行 JSON，不认识的字段不管，错误统一成 `Refusal{code,message}`；走共用的 `command::run`（10 秒超时）。`denials` 的 `not_found` 当“还没有数据”。后台的 `Watcher` 每 3 秒读一遍套墙 agent 的 `denials --limit 5000`，每 60 秒重新找一次 fence；`Seen` 留着最新结果给界面画。
2. **New Agent**（`new_agent.rs`、`new_agent_view.rs`）：角色按钮左边的锁和档案菜单（None + 各档案：名字、`built in`／`yours`、说明原文、替换内置、读不了的原因；底部检查结果和 `See the rules` 打开 Settings › Sandbox）；选了档案跑 `fence check`，换档案／种类立即、改目录等 250ms；警告黄点 + 底栏下原文，失败红锁 + 错误码和原话、Start 和 ⌘↩ 不能按；Controller 灰锁和说明，切到 Controller 回 None；不支持的种类灰锁回 None；`--sandbox <档案>` 进参数、Will run 可见；corral 回 `sandbox_*` 时把错误码、message 和 fence 的错误原话当启动失败显示。
3. **预设**：`agent_presets` 多可选 `sandbox`，只存 Regular 的；改锁算改了预设；套用时档案不在／种类不支持回 None。旧配置照读（测试覆盖）。
4. **卡片**（`card.rs`、`sidebar.rs`）：名字和强度后的暗色小锁、黄色计数、悬停文字；展开后横跨两栏的 Sandbox 格，按下开浮窗；锁和计数的宽度算进名字行余量。
5. **被拦记录浮窗**（新 `app/src/sidebar_sandbox.rs`，`Floating::Denials`）：标题、名字 · 档案、条数、关闭；`Network & writes`／`All` 切换；每行时间、彩色类型签、`~` 缩短的目标（悬停全路径；`target` 为空显示 `detail`）、`detail`；新的在上；空、读不了、`truncated` 各有一行；随读取刷新；底部固定说明。
6. **Settings › Sandbox**（新 `app/src/settings_sandbox.rs`，`Page::Sandbox` 在 Instructions 和 Diagnostics 之间，命令面板也能搜到）：版本状态、档案列表、本机补充文件；选档案、种类、目录后显示来源层、允许域名、不能读／写、能写／读（先 5 条可展开）、跳过项、警告；`View only. Profiles are changed in fence, not here.`；没装 fence 一句话。
7. `docs/DESIGN.md` §13 加了 P5-120 一条（决定和理由）。
另外：`footer_icon` 加了锁的画法（`Icon::Lock`、`lock()`）；`right_panel::Tip` 文字改成 `SharedString` 以便放动态文字；`corral::Agent` 多 `sandbox` 字段。

### 验证了什么
- `git diff --check` 通过；`app/` 下 `cargo test --all-targets` 786 个全过，`cargo clippy --all-targets -- -D warnings` 无告警，`cargo fmt --check` 通过。
- 新测试：fence 解析和调用（假 `fence` 脚本：版本约定号、profiles、check 及其拒绝、denials 和 `not_found`、后台 Watcher 找 fence 并读各实例、fence 消失时清掉记录；没写超时的测试）；New Agent 的 sandbox 规则（Controller、不支持的种类、预设存取和失效、`--sandbox` 位置、corral 的 `sandbox_failed` 文字）；卡片计数／悬停／Sandbox 格文字（含没 fence、读别的实例、读不了、`N+`）；浮窗的排序过滤、行的 `~` 和 `detail` 兜底、本地时间；只读套墙 agent 的实例；Settings 页的路径显示。
- 拿本机真的 `fence --version`、`fence profiles`、`fence check --profile dev --kind claude --cwd /tmp`（只读）对照过字段，和解析结构一致。
- 没起 paddock 窗口、没截图；界面样子、点选、键盘留给用户装好后看。

### 拿主意的地方
- 计数拿满 5000 条（`--limit 5000`）时写 `N+`。
- 只要 fence 找不到（或约定号不对），卡片也不画锁，即使 corral 说 agent 套着墙——和“没 fence 时任何地方都没有 sandbox 界面”一致。
- `--sandbox` 放在标签后、`--prompt`／`--` 前，不挪 cwd 在参数中的位置（别的代码按位置取它）。
- 没装 fence 时套用预设，保留预设里的 sandbox 不清掉，免得一次没找到 fence 就把预设改成“已编辑”。
- 档案有 `error` 也能选，交给 `fence check` 去拒（拒了 Start 不能按），菜单里标出原因。
- 浮窗每次打开回到 `Network & writes`；`All` 旁加一句小字 `All adds reads and system checks, mostly caches`。
- Settings 里 `<运行时才有>` 显示成 `temp dir (made at start)`（界面文字用英文）。
- 不支持的种类锁的悬停文字 `fence has no sandbox for <Kind>`；锁有警告时悬停 `Sandbox: <档案>, with a warning`。

### 没做的事
- 照「不要做」：没改 ranch，没做 C2、改档案、退出码 125 的提示、远程 sandbox、paddock ctl／看板开 agent 的路子；没用 `--sandbox` 真起 agent，没碰任何现有 agent。
- 没做界面实测（不能起窗口），样子和交互待用户看。

### 踩过的坑
- 用 perl 的 `s|…|…|` 改含 `|`（闭包、`||`）的代码时替换失败，还把垃圾写到了文件开头，后来改用 Edit 或别的分隔符重做。
- 中途误跑过一次 `python3 --version`（只是看版本，没用 Python 写或跑任何东西），之后没再用。
- Settings 多一页后命令面板里一条“Settings 页数为 6”的老断言跟着改成 7。

## 返工 1（交叉审查，见主仓库 `docs/任务/P5-120-交叉审查.md`「审查意见」）

### 改了什么
- **必须改 1：手改成自定义程序仍带 sandbox。**`Form` 加 `runs_kind()`（Command 解析出的种类就是当前种类）；能不能选锁、选了的留不留、参数里带不带 `--sandbox`、预设存不存，都以“当前确实能套”（`sandbox_now()`：Regular、认得的种类、fence 给了这个档案）为准。`set_command` 每次都重新核对，不再只在种类变化时。自定义程序时锁灰，悬停 `No sandbox for this command: fence's rules are by kind`；改回认得的种类不自动恢复。
- **必须改 2：检查没出结果就能 Start。**新的 `start_waits()`：选了 sandbox 时，只有当前“档案 + 种类 + 目录”的检查明确通过才放行；没查、正在查（含 250ms 等待）、失败都拦住。另有 `launch()`（等着时不给参数）；Start 按钮和 ⌘↩（`create`）共用。没选 sandbox 时照旧。
- **必须改 3：fence 还没回答时套用预设会丢 sandbox。**`Form` 分开“还在找 fence”（`looking`）和“没有 fence”；这时套用带 sandbox 的 Regular 预设，选择先记在 `pending` 里：预设不算改过，Start 等着（`sandbox_pending()`），fence 回答且允许时用上；等待期间切 Controller、换种类（菜单或手改命令）、选 None，都以用户的为准、丢掉 pending。fence 最后没找到时照旧不清预设里的值。
- **必须改 4：Watcher 释放时等满 10 秒。**fence 的四个调用都接一个取消信号，一路传给 `command::run`（照旧经过 `spawn_guard`）；Watcher 把自己的取消信号传进去，释放时正在跑的 fence 被杀掉并回收。
- **建议改 5：过期检查继续跑。**新增 `fence::Cancel`（丢掉即取消）。New Agent 的检查对象自己带一个，换新检查、回到 None、重新找 fence 时旧的立即停；查找 fence 也带一个。关面板（Esc、⌘W、点外面）时停掉在跑的检查和查找。Settings › Sandbox 同样：换选择停旧检查，重进页面停旧查找，离开这一页两样都停。被停掉的调用的回答不采用（否则被取消的 `--version` 会被当成“没装”）。
- **建议改 6：浮窗看不到全文。**每行悬停一律显示完整目标和 `detail` 原文（两者都有时分两行），不再只在缩成 `~` 时才有；`target` 为空时悬停显示完整 `detail`。
- **建议改 7：拿满 5000 条筛完为 0。**卡片数字写 `0+`（悬停 `· 0+ blocked`，Sandbox 格 `0+ blocked ›`）；浮窗空列表在这种情况下说 `None among the latest 5000 records; older ones aren't read.`，没拿满时照旧 `Nothing blocked yet.`。
- **第 8 条（只记录）：**DESIGN P5-120 那条写明：退回旧版 paddock 之前要先去掉配置里各预设的 `sandbox` 字段（或恢复旧配置备份），否则旧版整个配置读不了、启动报错。
- DESIGN §13 P5-120 那条按上面各项同步改了。

### 验证了什么
- 补的测试，改前红、改后绿（都跑过）：
  - 1：`new_agent::tests::a_command_paddock_cant_read_as_a_kind_has_no_sandbox`。改前在“改成 `my-agent --flag` 后 sandbox 应为 None”处红（实际是 `Some("dev")`）。另外覆盖：不能再选、参数里没有 `--sandbox`、直接往字段写也不进参数、改回 Claude 不恢复、Controller 的参数一定不带（审查指出单删参数层的 Controller 防护原测试抓不到，这里补上了）。
  - 2：`new_agent_view::tests::start_waits_for_fence_to_pass_the_sandbox_as_the_form_is_now`。用假 corral 记调用次数，走和 ⌘↩、Start 相同的 `launch()`：不选 sandbox 能起；没查、正在查、失败三种状态 corral 调用都是 0；通过后起一次且命令里有 `--sandbox dev --`；之后改目录又拦住、调用数不变。门槛是先改的，所以临时把 `start_waits` 换回旧逻辑（只拦已失败）跑了一次，确认它红（“没查”那一步就红了），再恢复。
  - 3：`new_agent::tests::a_preset_picked_while_fence_is_looked_for_gets_its_sandbox_once_found`。先 pick 带 dev 的预设、后收到 offer，以及等待期间切 Controller、换种类、选 None、fence 没找到、档案不在等情况。改前在 `sandbox_pending()` 处红：先放了一个永远返回 false 的空函数让测试能编译，红的原因是行为不对，不是编译错误。
  - 4：`fence::tests::dropping_the_watcher_stops_the_fence_it_is_waiting_on`。假 fence 记下 PID 后卡住；释放 Watcher 要在 3 秒内返回，且那个 PID 已不存在（被杀并回收）。改前红：等了 9.6 秒。
  - 5：`fence::tests::a_check_no_longer_wanted_stops_at_once`（慢的假 fence，丢掉 `Cancel` 后 3 秒内返回错误、子进程已回收）。这是新接口，没有“改前”可跑。
  - 6：`sidebar_sandbox` 的行测试补了家目录外的长路径、`target` 为空的长 detail，悬停内容都是全文。
  - 7：卡片测试补了 `(0, true)` → `0+`／`0+ blocked ›`（改前红：实际是 None）；`an_empty_list_of_a_full_read_doesnt_say_nothing_was_blocked`。
- `app/` 下 `cargo test --all-targets` 792 个全过，`cargo clippy --all-targets -- -D warnings` 退出码 0，`cargo fmt --check` 通过；`git diff --check` 通过。
- 没起窗口、没截图、没碰 agent、没用 `--sandbox` 起 agent。

### 拿主意的地方
- “认得的种类”沿用 New Agent 原有的命令解析（第一个词的程序名是不是六种之一）。所以 `env X=1 claude` 这种包一层的写法也算认不出、不能套。
- 等 fence 回答时，锁照旧不出现（还没有档案可列）；Start 因 pending 灰着。通常几十毫秒，没另加提示。
- 卡片的 `0+` 和别的数字一样用黄色。

### 踩过的坑
- 中途误敲了一个会等标准输入的 `cat >> /tmp/null`（违反“前台命令别带等标准输入的东西”），命令被挂到后台，发现后用 TaskStop 停掉。它在 `/tmp/null` 留了一个空文件；因为不能用 `rm`，没删，留给主控处理。代码没被改动（同一行后面的 perl 没跑）。
- perl 替换又碰到分隔符冲突（这次是 `#` 和 `#[test]`），改用 Edit。
