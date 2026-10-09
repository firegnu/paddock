# 任务：Agent shell——开一个普通 shell，里面敲 claude／codex 就由 corral 托管

2026-10-09，paddock/main 交给 paddock/dev-agentshell（Codex，重：gpt-6-astra / xhigh）。
路由：重 / 交叉审查要 / 影响面：碰要害（路由：档重 1.0，交叉审查要，碰要害；安全隐私 0.93：整份环境交给 agent、新的 ctl 请求会换掉窗格）
类型：功能变更
依据：
- 用户 10-08：“现在新建agent现在有点繁琐，还要自己选。一般来说herdr中就是新建agent的时候，就是弹出一个terminal,然后用户在terminal中输入codex --yolo， claude，pi之类的。直接就跑起来了。我想增加这么一个上面我描述的新建agent的流程，现在的新增agent的保持不变。”“对，要的是corral托管，而不是那种直接从命令行启动的agent。”
- 用户 10-09：选真 shell；入口放新建标签／分屏面板里一行；“我觉得就是普通的shell，就像herdr那种的。new agent之后就是开一个普通的shell，然后这个shell就是正常的terminal，用户可以当成正常的terminal使用，然后可以codex，claude等命令直接启动，但是这个后台你得默认让corral托管。”
- 调研 `docs/调研/P5-61r-agent-shell.md`（主控实测），用户看过后定：起来后**换成 agent 窗格**；名字**全自动**；一次性用法**放行**；**把 shell 的整个环境传过去**。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md` 全文（规矩：不用 Python、依赖固定、独立编译目录、不起窗口等）。
- `docs/调研/P5-61r-agent-shell.md` 全文：做法、实测、坑都在这里，照它做。
- `docs/DESIGN.md` §13 的 P5-39（paddock ctl）、P5-47（agent 停了窗格自动关）、P5-66（退出确认）几条。
- `app/src/new_agent.rs`：`suggest_prefix`、`read`（从命令读 model、effort）、`Form::args`（New Agent 怎么拼 `corral start`）；`app/src/window.rs`：新建标签／分屏面板（`Choice`、`choices`）、`shell()`、`NewShell`、窗格换内容的做法；`app/src/control*.rs`、`app/src/control_cli.rs`：ctl 的请求、调用者窗格定位；`app/src/pty.rs`：shell 怎么起、身份变量；`app/src/main.rs`：程序入口。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-61-agent-shell`，分支 `p5-61-agent-shell`（已从 main 建好）。
- 动 `app/src/` 下需要的文件（新模块放 `app/src/agent_shell.rs` 一类，`lib.rs` 登记）、`app/tests/`、`docs/DESIGN.md` §13，和本文件末尾的完成记录。不加新依赖。
- 编译目录：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-61-agent-shell`（已备好，增量编译）。

## 要做的
1. **入口**：新建标签／分屏面板（Shell、Agent… 那一列）加一行 “Agent shell”，副标题说明 claude、codex、pi、omp 会交给 corral。用户的 shell 不是 zsh 或 bash 时这一行变灰、悬停说明只支持这两种。开在新标签或分屏里，和 Shell 一样。New Agent 浮层不变。
2. **起 shell**：照普通 Shell 起用户的 shell，另加：
   - zsh：`ZDOTDIR` 指向 paddock 的一个目录，里面的 `.zshenv` 把 `ZDOTDIR` 改回用户原来的（原来没设就是 `$HOME`）、`source` 用户的 `.zshenv`；交互式时 `add-zsh-hook` 挂 `precmd`、`preexec`，每次把替身目录挪到 PATH 第一位（调研第 1 条实测过的写法）。
   - bash：`--rcfile <paddock 的文件> -i`，先 `source ~/.bashrc`（照 bash 交互式的读法），再用 `PROMPT_COMMAND` 每次挪 PATH。
   - 这些文件和替身目录放在 paddock 自己的私有目录（权限 0700，照 ctl 目录的做法），每次启动写一遍；路径不进用户的任何配置文件。普通 Shell 完全不变。
3. **替身**：`paddock` 程序按自己的程序名分派：以 `claude`、`codex`、`pi`、`omp` 的名字运行时走替身逻辑（替身目录里放指向 paddock 程序的符号链接），在 GPUI 初始化之前分出去。
   - 找真程序：PATH 里跳过替身目录找下一个同名可执行文件；找不到就照 shell 的样子报 command not found、退出 127。
   - **放行**（直接 `exec` 真程序，参数原样）：标准输入或标准输出不是终端；参数里有 `-p`／`--print`、`--version`、`-v`、`--help`、`-h`；第一个参数是子命令（claude：`mcp`、`config`、`update`、`doctor`、`install`、`migrate-installer`、`setup-token`；codex：`exec`、`e`、`login`、`logout`、`mcp`、`mcp-server`、`apply`、`a`、`completion`、`debug`、`proto`；pi、omp 只按前两种规则）。这张表写在一处、带测试。
   - **交给 corral**：名字 `<前缀>/<种类>` 加 `--unique`（前缀用 `suggest_prefix`：当前目录所在 git 仓库的顶层目录名，不在仓库里用当前目录名）；`--cwd` 当前目录；`--label role=regular`，能从命令读出 model、effort 的加 `--label model=…`、`--label effort=…`（复用 `new_agent::read`）；**整个环境**用 `--env K=V` 传过去，去掉排除单（`CORRAL_*`、`PADDOCK_*`、`SADDLE_*`、`TERM`、`TERM_PROGRAM`、`TERM_PROGRAM_VERSION`、`SHLVL`、`PWD`、`OLDPWD`、`_`，以及替身目录从 PATH 里拿掉）；`--` 之后是真程序的**绝对路径**和原参数。corral 程序由 paddock 通过环境变量告诉替身（配置里的 `corral`）。
   - 实测确认（调研第 8 条留下的）：`--env PATH=…` 能盖过 corral 自己的 PATH；整个环境的参数长度没问题（太长时报清楚的错，不静默丢变量）。
   - 起好后：经 `paddock ctl` 请 paddock 把**调用者这个窗格**（`PADDOCK_INSTANCE`、`PADDOCK_PANE`）换成显示这个 agent 的普通 agent 窗格（和 New Agent 开的一样，P5-47 等照常），替换时结束原来的 shell、**不问**（P5-66 的确认不适用于这一步）。ctl 不通（不在 paddock 里、paddock 已退出）：替身在前台 `corral attach <名字>`，退出码照 attach 的，并在开头打一行说明。
   - `corral start` 失败：打出 corral 的错误，退出非 0，留在 shell 里。
4. **ctl 新请求**：只能把**发请求的那个窗格**换成 agent（按现有的调用者定位核对，不能指定别的窗格）；agent 名字必须是 `corral ls` 里有的；窗格不是 Agent shell 开的就拒绝。写进 ctl 的帮助和测试。
5. **DESIGN §13** 加 P5-61 一条（入口、做法、四个用户决定、只支持 zsh／bash、坑：环境不继承、一次性放行）。

## 怎么算做完
- 用户原话：“就像herdr那种的……用户可以当成正常的terminal使用，然后可以codex，claude等命令直接启动，但是这个后台你得默认让corral托管”；用户定的四条（换成 agent 窗格、全自动名字、一次性放行、整个环境）。
- 测试（先写、确认因缺功能失败再实现）：
  - 放行表：每条规则各一例，含不在终端里。
  - 找真程序跳过替身目录、找不到时 127。
  - 环境：排除单生效、替身目录从 PATH 拿掉、其余原样进 `--env`。
  - `corral start` 参数：名字、`--unique`、`--cwd`、标签、真程序绝对路径和原参数（用假 corral 脚本录下参数，脚本用 sh，不用 Python）。
  - 真 pty 测试（照 P5-66 的 pty 测试做法）：临时 HOME 里放一个会改 PATH 的 `.zshrc`（和 `.bashrc`），用 paddock 的 ZDOTDIR／rcfile 起交互 zsh、bash，敲 `command -v claude` 得到替身路径；普通 Shell 起的得到的不是替身。
  - ctl：只能换调用者自己的窗格、名字不在 `corral ls` 里拒绝、非 Agent shell 窗格拒绝。
  - 边角（碰要害这一档要）：用户原来设了 `ZDOTDIR`；PATH 里替身目录出现多次；环境值里有换行和等号。
- 验证只做这些：上面的测试；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 各跑一次。实测真 agent 只在必要时用自己开的 `paddock/test-<名字>`（`--label role=test`），用完 `corral stop`。觉得不够，在回复里说，不要自己加。
- 真窗口里的手感留给用户。

## 不要做
- **不要起 paddock 窗口**（`cargo run` 主程序、截图、实测都不要）：10-09 出过事故，在 agent 会话里起的测试窗口会和所有 agent 归进同一个进程组，从 Dock 上退出它把全部 agent 一起结束了（见 `docs/任务/P5-52-Dock里第二个paddock进程.md`）。
- **不用 Python**（命令、脚本、测试都不用；仓库规矩）。命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带会等标准输入的东西。
- 不读、不改用户的 rc 文件和任何配置；测试一律用临时 HOME。
- 不把环境变量的值写进日志、错误信息或测试快照（环境里可能有密钥）；报错只写变量名。
- 不改 New Agent 浮层、普通 Shell 的行为。
- 不加新依赖。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）；不要对 `corral ls` 里别人的 agent 做 stop、send、keys。
- 遇到做法和调研对不上、要改 corral（ranch）、或要改已定的设计，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-61-agent-shell` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-10-09，p5-61-agent-shell。

- 做了什么：新建标签／分屏面板增加 Agent shell 和说明；非 zsh／bash 时禁用并给悬停说明。新增 `agent_shell.rs`，在 GPUI 初始化前按程序名分派四种替身；私有目录沿用 ctl 的权限检查，按 paddock 进程隔离，启动时原子重写 rc 文件和符号链接。普通 Shell 和 New Agent 浮层维持原路径。托管启动复用名字清理、model／effort 读取规则，传真程序绝对路径、原参数和过滤后的完整环境；一次性命令直接 exec，找不到程序退出 127，start 失败返回原 shell；过长环境明确报错，不丢变量，错误中的环境值作遮盖。
- 做了什么（ctl）：新增 `paddock ctl agent-shell NAME` 及帮助，只能替换调用者自己的、仍活着的 Agent shell。先查调用者，再异步读公开 `corral ls`，返回后复查窗格 revision、身份、类型及 agent 名单，随后用现有 attach 路径替换；不会询问关闭 shell，也不移动焦点。请求和进度沿用既有记录机制。ctl 不通或拒绝接管时说明后在当前终端前台 attach，返回它的退出码。DESIGN §13 已记录四项用户决定及边界。
- 验证了什么：放行表、真程序查找／127、假 corral 记录启动参数及环境、zsh／bash 真 PTY 钩子、ctl 身份计划／命令行／socket 通信均保留功能性 RED→GREEN；测试修正过长 socket 路径及权限的夹具失败不计 RED。覆盖自定义 ZDOTDIR、rc 和后续命令改 PATH、重复替身目录、含换行和等号及空值的环境、git 子目录取顶层名字、start 失败返回仍活着的 shell及错误遮盖、操作系统参数长度上限。纯 UI 入口通过源码与原选择器回归检查；新增行导致的键盘滚动索引错位也先复现再修正。
- 验证了什么（规定检查）：在独立编译目录前台完成 `app/` 的 `cargo test --all-targets`（496 项通过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，各一次。收尾发现既有 Bash PROMPT_COMMAND 以分号结尾时拼接会失效，补入同一 PTY 测试确认 RED，改为换行拼接后，6 项 Agent shell 集成测试全部通过；没有重复全量检查。最后 `git diff --check` 通过。未放宽断言、修改超时或增加依赖。
- 实测：只用自己创建、带 `role=test` 的 `paddock/test-p5-61-env-*` 实例运行合成 `/bin/sh` 探针，确认真实 corral 的 `--env PATH=…` 覆盖生效、65,536 字节合成环境值完整到达；随后 `corral stop` 成功，临时探针目录已清理。没有启动真实 AI、接触其他 agent，环境值不进日志或快照。
- 拿主意的地方：新 agent 可能尚未进入侧栏缓存，所以 handoff 单独刷新公开名单；在途检查返回后重新核对窗格，避免替换已被用户换掉的内容。私有脚本按进程隔离，避免同时运行的 paddock 互相改替身指向。Bash 保留已有 PROMPT_COMMAND，最后再提升替身 PATH。布局格式不扩展，尚未起 agent 的 Agent shell 保存、恢复时沿用普通 Shell。
- 没做的事：没有启动 paddock 窗口、截图、模拟系统输入，没有读改用户 rc／配置，没有改 ranch／Saddle，没有合并、推送、安装。真窗口里的手感与任务指定的交叉审查留给主控安排；没有新增需要主控决定的设计问题。

### 主控补充核对：只新增，不改已有流程（2026-10-09）

用户原话：“现在做的这一套是新增的一套new agent入口。已经有的不能改哦”。按主控列出的三条逐项核对 `7679565` 相对其父提交的差异，本轮无需修改产品代码，只补完成记录。

- 入口和键盘：`choices` 明确是 Shell、Agent…、Agent shell、已有 agent；前两行的渲染、点击处理与原来一致。`agent_matches`、`first_choice`、上下键 `move_selection`、回车 `open_selected` 的逻辑未改；只为插入一行调整标题后的滚动索引和列表计数。已有筛选测试仍确认大小写／多词匹配、输入后选首个匹配 agent、无匹配回到 Shell；滚动测试仍确认上下键定位到选中行，前两行位置不变。
- New Agent／普通 Shell：`new_agent.rs`、`new_agent_view.rs`、`pty.rs`、`menu.rs`、`layout_state.rs` 与任务前完全无差异。普通入口、恢复布局及旧 ctl 的 Shell 路径均设 `agent_shell=false`，沿用原来的 `[program, "-i"]`、cwd、身份环境和 `Session::spawn_shell`；只有新入口设 true 并注入替身配置。原浮层、快捷键及布局格式未改。
- 旧 ctl：只新增 `Operation::AgentShell`／`agent-shell NAME` 及对应处理；已有 inspect、instances、open、close、browse、request 的参数规则、响应构造和 JSON 字段未改，内部 `Facts.agent_shell` 不写入原有响应。帮助仍是原 JSON 格式，仅追加新请求说明。既有 open 参数、inspect 输出、身份传递、请求重放／进度及 close 确认测试通过。
- P5-47／P5-66：原 agent 结束自动关窗格、最后一格补普通 shell、退出／关闭确认及程序探测逻辑无改动。无确认替换仅在新增 AgentShell 请求通过调用者类型／身份检查后发生；普通 close 路径、其他 shell 的确认规则保持原样。已有 agent 结束／重开／暂停／接入中测试、关闭确认测试，以及真 PTY 的前后台程序探测测试通过。
- 本轮验证：全部命令在前台完成，复跑 `cargo test --lib control`（49）、`cargo test --lib window::tests::`（32）、`cargo test --lib new_agent::tests::`（18）、`cargo test --lib pty::tests::`（2）、`cargo test --test ctl_cli --test viewer`（3＋6），共 110 项通过；`git diff --check` 通过。未新增或放宽测试，未重复全量检查，未启动窗口、真实 agent、合并或推送。无新增待主控决定事项。

### 返工（2026-10-09）

按主仓库 `P5-61-审查.md` 的 R1、S1、S2 返工，三条均先改测试、确认目标行为失败，再修改实现。

- R1：PATH 清理时解析当前可执行文件和各目录下四种替身的符号链接，剔除仍指向当前 paddock 的其他替身目录；真程序查找、放行 exec 的 PATH 和托管 `--env PATH` 共用清理结果。新增“两套替身目录＋真程序”两条回归，第二套经第一套链接到 paddock。RED：放行在原有 5 秒测试时限内不退出，托管参数错误地指向第二套替身；GREEN：放行实际执行真程序并返回 23，原参数保留，托管传入真程序绝对路径，两条路径都只留下真实目录及系统目录。
- S1：失败夹具改为真实的 `ok:false`、`error`、`message` 三字段。RED：旧实现只显示错误码，缺少具体原因；GREEN：显示 `exec_failed` 和 message，拼接后仍统一遮盖环境值，合成敏感值不出现，返回非零且原 shell 继续可用。兼容仅 error／message 或 stderr 的诊断。
- S2：单独传递原 ZDOTDIR 是否存在的标记，显式空值不再回退 HOME；未设置时保留任务原定的 HOME 回退。新增临时 HOME 下普通 zsh／Agent shell、未设置／空值的四种对照。RED：显式空值错误读取 HOME 的 `.zshenv`、`.zshrc` 并改成 HOME；GREEN：空值保持为空、不读 HOME 的两个文件，未设置时仍读取它们；已有非空自定义 ZDOTDIR 测试也通过。
- 相关验证：逐条 RED→GREEN 后，前台运行 `cargo test --manifest-path app/Cargo.toml --test agent_shell`（9 项）和 `cargo test --manifest-path app/Cargo.toml --lib agent_shell::tests::`（2 项），全部通过。覆盖普通 zsh／bash 对照、rc 改 PATH、127、放行参数／退出码、环境传递、失败返回 shell、ctl 接管及过长环境；未放宽断言或修改超时。格式整理后 `cargo fmt --manifest-path app/Cargo.toml --check`、`git diff --check` 通过；按主控要求未重跑全套测试或 clippy。
- 范围核对：本次产品差异仅在 `app/src/agent_shell.rs`，另外只改对应集成测试和本完成记录。入口排序／筛选／键盘、New Agent 浮层、普通 Shell 实现、既有 ctl 命令和输出格式、P5-47／P5-66 代码均未改；上节已通过的 110 项兼容回归记录仍保留。
- 没做的事：未启动窗口或真实 agent，未读改用户 rc／配置，未动主仓库、ranch 或 Saddle，未合并或推送。只在本分支提交，无新增需主控决定事项。
