# 任务：远程 New Agent 对话框里自己配 agent 的命令

2026-10-17，paddock/main 交给 paddock/dev-remote-command（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：拿不准、拿不准、拿不准，档位倾向常规；主控按规则定：对话框加一行和本机状态文件，沿用 P5-124 的远端起法和引号做法，不改用户配置、不动已在跑的 agent）
类型：功能变更
依据：用户 10-17 试 P5-124 后：“我觉得是不是可以诶用户提供一个自己配置远程agent的方式（remote new agent那个对哈框中）。这个你考虑一下”；主控给了三种放法，用户选“Command 行 + Custom…胶囊”，顺序选“先修 bug 再做自定义”（P5-124b 已合并）。看过样稿（画布「P5-124 远程开 agent 样稿」的 F、G，源文件在 `docs/设计稿/P5-124-远程开agent/`）和主控的六条提议：“可以。按照设计做吧，但是要精致”。
依赖：P5-124b
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
优先：高
你是被委派的 agent：照本文件做，不要再开别的 agent。

派出的理由：对话框的新一行、新胶囊和状态文件，界面实现按分工给 Claude Code。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- 样稿：`docs/设计稿/P5-124-远程开agent/README.md`，尤其 `F-Command.dc.html`、`G-Custom.dc.html`；`C-Dialog`、`D-States` 是现在的样子。
- `docs/DESIGN.md` §13 P5-124 那条（含 P5-124b 的交互登录 shell）。
- 代码：`app/src/remote_agent_view.rs`（对话框）、`app/src/remote_start.rs`（`Request`、`login`、`look`、`RecentDirs`）、`app/tests/remote_start.rs`；本机 New Agent 的 Command 怎么拆词：`app/src/new_agent.rs` 的 `Form::args`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-124c-remote-command`，分支 `p5-124c-remote-command`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-124c-remote-command`（已从 `.target/main` 克隆）。
- 动 `app/src/` 下要动的文件、相关测试、`docs/DESIGN.md` 和本文件。不加依赖。

## 要做的（照样稿 F、G；颜色从主题取，样稿色值只是示意）
1. **Command 行**：在 Agent 和 Directory 之间加一行 `Command`，显示实际要跑的命令。选探测到的那种时预填它的命令（`Tool::command()`，如 `pi`、`codex --yolo`），可以直接改；改过（和默认不同）时下面写 `Runs through your login shell on <显示名>. Edited, kept for <显示名> ·` 加一个 `Reset to <种类>` 链接，点了回到默认；没改过只写前一句。
2. **`+ Custom…`**：Agent 那排最后加一颗 `+ Custom…`。选它时 Command 是空的，占位字 `amp, or ~/bin/my-agent --flag`，下面一句写起成功后会记成哪颗胶囊（样稿 G 的原文）。Command 空着时 Start 不能点。Agent 那排下面那句改成 `Found on <显示名>, and the ones you added.`。
3. **记住**：只在 corral 答起成功以后才记，按机器分开（键用机器的 `name`），存在本机状态文件里（和最近目录放一处、互不干扰），不写 `config.toml`。
   - 探测到的那种改过的命令记在那一种上，下次选它预填改过的；Reset 回默认后也记下（等于删掉改过的）。
   - 自定义的变成一颗胶囊，排在探测到的后面、`+ Custom…` 前面；名字取命令第一个词的最后一段（`~/bin/my-agent --fast` → `my-agent`），同名的后来者替换先来的；图标用虚线框的通用图标（样稿 F），第一个词正好是已知种类（如 `amp`）时用那一种的图标。悬停出 `×`，点了从这台机器的记录里删掉（不用确认）；删的正是选中的那颗时，选回第一颗。
   - 自定义的不去远端探测，一直列着；探测到的照 P5-124/P5-124b 每次打开现查。
4. **起法**：Command 照 shell 写法拆词（同本机 New Agent 的 Command，可以带引号），拆不开（引号没配对）时在 Command 下面当场提示；拆出的每个词再照现有的 `command_line`/`quote` 引起来，经 P5-124b 的交互登录 shell 跑 `corral start … -- <这些词>`。名字留空时用 `<目录末段>/<胶囊名或种类>`。起不来时 corral 的错照样显示在框里。
5. amp、agy 的特殊处理（插件、第一句话）不做：自己填 `amp` 能起，第一句话、插件提示照旧没有。

### 精致（用户：“但是要精致”，和 P5-124 同一套标准）
1. **对齐和尺寸**：Command 和其他三行标签列等宽、输入框左缘对齐、行距一致；胶囊多到一行放不下时整齐换行，不撑宽对话框；尺寸字号走现有界面缩放，深浅两种主题都成立。
2. **状态齐全**：胶囊（含自定义的和 `+ Custom…`）、`×`、`Reset to …` 链接都有悬停、按下、键盘焦点、不能点的样子；`×` 只在悬停那颗或它有键盘焦点时出现，出现时胶囊宽度不跳。
3. **键盘**：Tab／⇧Tab 走 Agent → Command → Directory → Name；←→ 在胶囊间换（含自定义的和 `+ Custom…`）；选 `+ Custom…` 后焦点落到 Command；⌘↩ 开始、Esc 取消；中文输入法选字时的 Return 归输入法。
4. **不跳**：换种类时 Command 的内容和下面那句跟着换，对话框宽度不变、按钮不挪；长命令在框里横向滚动或换行都行，但不撑破对话框、不被截掉看不到光标。

## 怎么算做完
- 用户原话：“我觉得是不是可以诶用户提供一个自己配置远程agent的方式（remote new agent那个对哈框中）”；选的“Command 行 + Custom…胶囊”；“可以。按照设计做吧，但是要精致”——样子和样稿 F、G 一致，并做到上面「精致」四条。
- 验证只做这些：针对这件活写测试，覆盖 Command 拆词后再引号（带空格、单双引号、`$`、`;`、反引号的命令原样到 corral，不被远端 shell 当命令执行；引号没配对时报错）、胶囊名的取法、记住的规则（起成功才记、按机器分开、改过的命令记在那一种上、Reset 删掉改过的、同名自定义替换、删掉一颗）、状态文件读写（读不到或坏了当成空的）、选 `+ Custom…` 且 Command 空时不能起。集成测试照 `tests/remote_start.rs` 用假 ssh、假登录 shell、假 corral。`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑过一次。觉得不够，在回复里说，不要自己加。
- `docs/DESIGN.md` §13 P5-124 那条后面补一条 P5-124c，写清 Command 行、`+ Custom…`、记在哪、怎么起。

## 不要做
- 不连远程机器（包括 5090），不跑真实 ssh；不对任何机器上 `corral ls` 里的 agent 做 `stop`、`send`、`keys`、attach；不做任何断网、断线测试。
- 不改、不读用户真实的 `~/.config/paddock/config.toml` 和本机真实的状态文件，测试一律用临时目录；不起 paddock 窗口截图（样子留给用户看）。
- 集成测试不要用本机真的登录 shell（会读本机配置，把本机装的 agent 当成远端的），用假登录 shell。
- 不改本机 New Agent；不改 P5-124/P5-124b 的入口、探测、交互登录 shell、目录和名字的规则；不加 amp、agy 的插件提示和第一句话；不做 Settings 里配命令。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程。
- 遇到要改公开约定（`corral`、`ranch` 的命令或输出）才能做的，停下来报告。
- 不合并到 main，不推送。只在 `p5-124c-remote-command` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的，工具或环境和预想不一样的；没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么
- `app/src/remote_start.rs`：`Request` 改成 `agent`（`Agent::Kind(种类)` 或 `Agent::Custom`）+ `command`（照打的字）+ 目录 + 名字。`check_command` 照 shell 写法拆词（`shell_words`，同本机 `Form::args`），引号没配对、空、带控制字符时报到 `Field::Command`，排在目录之前；`start` 先查问题，有问题不连远端。拆出的词开头的 `~`、`~/` 换成那台的 HOME 再照 `command_line`/`quote` 引起来，经 P5-124b 的交互登录 shell 跑 `corral start … -- <这些词>`。`custom_name`（第一个词最后一段，按名字前缀的规则）、`edited`（和种类自己的命令逐词比）、`default_name` 改成收胶囊名或种类。新 `Commands`（`remote-agent-commands.json`，在 `recent-remote-dirs.json` 旁边、各是各的文件）：按机器 `name` 记 `edited`、`custom`；`remember`／`forget`／`customs`；`keep` 只在起成功后同时记最近目录和命令。存文件的写法和最近目录共用一个 `save_json`。
- `app/src/remote_agent_view.rs`：照样稿 F、G 加 Command 行、自己加的胶囊（虚线图标或已知种类的图标、`×`）、`+ Custom…`、`Edited, kept for … · Reset to …`、`Once it starts, … is kept for … next to the others.`、Agent 下那句 `Found on …, and the ones you added.`；凹槽能换行；对话框宽 480 → 560（样稿 F、G 的宽度）；键盘：Tab 走 Agent → Command →（有时）Reset → Directory → Name，←→ 走所有胶囊，选 `+ Custom…` 焦点到 Command，凹槽上 ⌫／Delete 删选中的自己加的那颗，Reset 链接上 Return／Space；`Starting…` 时胶囊、`×`、Reset 变淡不响应。
- `app/tests/remote_start.rs`：老的三处改成新 `Request`，加四个集成测试。
- `docs/DESIGN.md` §13 P5-124 后补了 P5-124c 一条。

### 验证了什么
- 单元测试（`remote_start.rs`，新加 8 个）：拆词后再引号（空格、单双引号、`$`、`;`、反引号原样，`~` 换成远端 HOME，`x~/y` 不动）；引号没配对、空、控制字符报到 Command 上且排在目录前；有问题的请求不发出去；胶囊名的取法和图标；逐词比较算不算改过；记住的规则（按机器分开、改过的记在那一种上、用回自己的命令删掉改过的、同名替换不挪位置、删一颗只删这台的）；状态文件读写、和最近目录互不干扰、坏文件当成空的、手改的读不成的略过。
- 集成测试（假 ssh、假登录 shell、假 corral，新加 4 个）：自己的命令原样到 corral、远端没执行 `touch`（三种注入写法都没有生成文件）、走了登录 profile；改过的种类命令照样起；引号没配对或空时一次 ssh 都没有发；起失败什么都不记、起成功才记。
- 为确认测试真能抓错：临时把“起失败也记”和“不换 `~`”两处改坏，对应的单元、集成测试各自变红，改回后全绿。（测试和实现是一起写的，没有先跑出红再写实现，用这个代替。）
- `git diff --check` 通过；`app/` 下 `cargo test --all-targets` 全过（lib 592 个等，0 失败），`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过。
- 没起窗口、没截图：样子、悬停、键盘、输入法都留给用户实际看。

### 拿主意的地方
- `~` 换成远端 HOME：corral 直接 exec 这些词、不经 shell（读了 `../ranch` 的 `pen.rs` 确认），样稿占位字就是 `~/bin/my-agent`，不换就起不来。每个词开头的 `~`、`~/` 都换（同 shell 对没加引号的 `~`）；代价是用户特意加引号的 `'~/x'` 也会被换，拆词以后分不出来。
- Command 下面固定两行（第一行 `Runs through your login shell on …`，第二行是改过／会记成什么／出错原因）：样稿把它们写在一行，但 G 那句在 560 宽里放不下一行，换胶囊时会一行两行来回跳、按钮上下挪，和「不跳」冲突。字照样稿，只是分两行。
- 对话框宽度跟 F、G 改成 560（C 是 480）。
- Agent 下那句：有自己加的才写 `, and the ones you added.`，一颗都没加时只写 `Found on <显示名>.`，免得说一句不存在的事。
- 已知种类的图标按胶囊名认（`/usr/local/bin/amp` 也用 amp 的图标），不只是第一个词正好等于 `amp`。
- 自己加的胶囊改命令：名字不变算改过（有 `Reset to <名字>`，起成功后替换它）；名字变了起成功后另成一颗，原来那颗留着。
- 在 `+ Custom…` 里写 `pi --x` 会变成一颗叫 `pi` 的自己加的胶囊，和探测到的 pi 并排（照「自定义的变成一颗胶囊」字面做的，没有改成记到探测到的 pi 上）。返工 1 已按主控的决定改掉，见下。
- 对话框开着时每颗胶囊打过的字各自留着，换回来还在，关掉就没了；没起成功的不记。
- `×` 的「键盘焦点」做成：凹槽有焦点、选中的正是这颗时 `×` 显示成淡强调色底，按 ⌫／Delete 删掉。`Reset to …` 进了 Tab 顺序（在 Command 和 Directory 之间，只在出现时），任务写的顺序里没有它，不进的话键盘碰不到它。
- 查种类的时候（`Checking…`）自己加的和 `+ Custom…` 已经能点；这时直接在 Command 里打字就当成 `+ Custom…`。没探测到也没自己加的时默认选 `+ Custom…`（焦点不动）。

### 没做的事
- 没连任何机器，没跑真实 ssh，没起 paddock 窗口、没截图；没读、没改真实的 `config.toml` 和状态文件。
- amp、agy 的插件和第一句话照旧不做；Settings 里配命令不做；本机 New Agent 没动。
- 界面画法（对齐、换行、深浅主题、悬停／按下／焦点的样子、输入法）没有自动测试，留给用户实际看。

### 踩过的坑
- 一开始误敲了一条带 `cat >` 的命令，在 `paddock-worktrees/` 下生成了一个空文件 `.p5c_patch_unused`，已移到本次会话的临时目录，worktree 外没有留下东西。
- `cargo test --all-targets <过滤>` 里 lib 测试一红就不跑集成测试了，验证改坏的那一处时改用 `--test remote_start` 单跑。

### 返工 1（主控 10-17 审查）
主控定：宽度 560、Command 下面固定两行同意不改；`+ Custom…` 里填的命令名字正好是这次探测到的种类时，要记成那一种的改过命令。
- 做了什么：`Commands::remember`、`remote_start::keep` 多收这次探测到的种类。自己写的命令名字（`custom_name`）是其中一种时，按在那颗胶囊上改命令处理（记成 `edited`，等于它自己的命令时删掉），不加自己的胶囊；没探测到才照旧记成自己的胶囊。新加 `Commands::shown` 和 `found_kind`：画胶囊时，名字和这次探测到的种类同名的那颗不画，记录留着，那一种没探测到时它又出来。对话框里的 `customs` 改成只放要画的（查完种类后再算一次）；查种类时点了这样一颗、查完被藏起时，改选同名的那一种。G 那句在这种情况下写 `Once it starts, it is kept as <种类>'s command for <显示名>.`（种类用 `Tool::label()`，如 `pi's`、`Codex's`）。`DESIGN.md` P5-124c 那条补了这一段。
- 验证了什么：新加单元测试 2 个（保存：探测到的同名记成改过、全路径同名也算、用回自己的命令删掉、没探测到或别的机器照旧成胶囊；显示：同名不画、那一种没探测到时又出来、记录不删），集成测试 1 个（假 ssh、假登录 shell、假 corral，种类用 `look` 真查出来的 claude、pi：`pi --model sonnet` 记到 pi 上、`~/bin/omp --x` 成胶囊、`pi` 删掉改过的）。两个单元测试先在旧行为上跑过，都失败；集成测试先写时有借用错误没编过，修好后临时把新规则关掉，它失败，恢复后通过。`remote_start` 单元 18 个、集成 9 个全过；`cargo clippy --all-targets -- -D warnings` 无警告；`cargo fmt --check` 通过（先跑 `cargo fmt` 重排了几行）；`git diff --check` 通过。按主控说的没重跑全套测试。
- 拿主意的地方：提示的措辞；全路径（`/opt/bin/claude`）按名字算同一种，这是 `custom_name` 本来的取法；查种类时选中的那颗被藏起时改选同名的那一种（不改的话选中的胶囊会消失）。
- 踩过的坑：追加集成测试的那条命令末尾误带了 `python3 -`，它等标准输入，被移到后台，立即停掉了；没执行任何 Python 代码，文件只追加了一次（查过）。

## 主控审查

2026-10-17，paddock/main。结论：返工 1 后可以合并。
- 第一轮看了 diff：只动了任务里的文件，「不要做」没碰；`check_command` 拆词、逐词 `quote` 再经交互登录 shell，`~` 换远端 HOME（corral 不经 shell 执行，读了 ranch 确认）——同意；只在起成功后记、按机器分开、存 `remote-agent-commands.json`、不写 `config.toml`，都对。
- 对方问的两件：宽度 560、Command 下面固定两行——同意，和样稿 F、G 一致、换种类不跳；`+ Custom…` 写探测到的种类会多出同名胶囊——交回返工：记成那一种的改过命令、显示时同名不另画。返工 1（`f20c18a`）照办，两个单元测试先在旧行为上红、集成测试临时关掉新规则确认红。
- 其余取舍同意：Agent 下那句没自己加时只写 `Found on <机器>.`；`Reset to …` 进 Tab 顺序；键盘选中自定义胶囊时 ⌫／Delete 删掉；查完种类后选中的自定义胶囊被藏起时改选同名那一种；全路径按名字算同一种。
- 重跑：`cargo test --all-targets` 全过（lib 594，加各集成测试），clippy、fmt、`git diff --check` 干净。
- 对方第一轮误建过一个空文件（已挪进它的临时目录），返工时一条命令末尾误带了 `python3 -`（在等输入，马上停掉，没有执行 Python 代码）。界面画法、深浅主题、键盘和输入法没有测试，留给用户实际看。
