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
