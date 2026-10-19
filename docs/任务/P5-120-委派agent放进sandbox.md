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
