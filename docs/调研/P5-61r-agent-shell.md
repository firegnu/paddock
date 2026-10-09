# P5-61r 调研：普通 shell 里敲 claude／codex 就由 corral 托管

2026-10-09，paddock/main。任务文件 `docs/任务/P5-61r-agent-shell调研.md`。只用临时 zsh／bash 会话和自己开的 `paddock/test-*` 实例实测，没起 paddock 窗口，没改用户的任何配置。用户的 rc 文件只看了“改不改 PATH、有没有给 agent 起别名”，内容不记在这里。

## 结论

能做，而且不依赖 zsh 的特殊功能：**在这个窗格的 shell 里，把 paddock 的同名“替身”放在 PATH 最前面**。用户照常敲 `codex --yolo`，shell 自己处理别名、变量、引号、`FOO=1` 前缀，最后执行到替身；替身在当前目录用 `corral start` 起真正的程序，再在同一窗格 `corral attach` 接上去。Ctrl-] 离开接入回到 shell 提示符，agent 照样在 corral 里跑；agent 结束时接入也结束，同样回到提示符。

## 逐项

1. **怎么让替身排在最前（已实测）**
   - 事实：用户的 `.zshrc` 有约 20 处改 PATH，`.zprofile` 4 处、`.zshenv` 1 处；所以只在启动时把替身目录放到 PATH 前面不够，rc 跑完会被挤到后面。
   - 做法：zsh 用一个 paddock 自己的 `ZDOTDIR`，里面的 `.zshenv` 先把 `ZDOTDIR` 改回用户自己的（照 VS Code、Ghostty 的 shell 集成做法）、`source` 用户的 `.zshenv`，交互式时用 `add-zsh-hook` 挂 `precmd` 和 `preexec`，每次都把替身目录挪到 PATH 第一位。用户的 `.zprofile`、`.zshrc`、`.zlogin` 照常从用户目录读。
   - 实测（`script` 起交互 zsh，用户真实配置，替身只打印参数）：`claude --model opus hello` → 到了替身；`alias cl="claude --effort high"` 后 `cl one` → 替身收到 `--effort high one`；`FOO=1 claude two` → 到了替身。
   - bash：`bash --rcfile <paddock 的文件> -i`，文件里先 `source ~/.bashrc`，再用 `PROMPT_COMMAND` 每次挪 PATH。实测 `claude bash-ok` → 到了替身。
   - fish：本机没装，没测；可以用 `fish --init-command` 加 `fish_prompt` 事件函数，做法同理。
   - 别的 shell：不拦，照普通 shell 开，入口上写明。
2. **替身怎么找到真程序**：在 PATH 里跳过替身目录，找下一个同名可执行文件。事实：本机 `codex` 本身是 `~/.ip_guardian_bin/codex` 这层包装，`claude` 在 `~/.local/bin`，`pi` 在 `~/.bun/bin`，`omp` 在 `/opt/homebrew/bin`；这样找到的就是用户平时用的那一个（包装保留）。
3. **交给 corral（已实测）**
   - `corral start <名字> --cwd <当前目录> -- /Users/…/.local/bin/claude`：corral 按程序的文件名认种类，绝对路径照样是 `kind: claude`；状态钩子照常装上（`SessionStart` 之后 `idle`）。所以替身传真程序的绝对路径即可，不用包一层 shell。
   - `--cwd` 就是 shell 当时的目录：用户可以先 `cd`、拉代码，再起 agent。
   - 接入：替身起完后在前台跑 `corral attach <名字>`（不用 `exec`，好让接入结束后回到 shell）。离开接入是 Ctrl-]（corral 的 detach 键）。
4. **环境变量（已实测，是个坑）**：`corral start` 起的 agent **不继承调用方的环境**——调用方设的变量、调用方改过的 PATH 都没到 agent 里，只有 `--env` 给的到了。所以 `FOO=1 claude`、shell 里 `export` 过的变量，要由替身用 `--env` 传过去，才像在普通终端里跑。建议：替身把自己的整个环境传过去，去掉一张小的排除单（`CORRAL_*`、`PADDOCK_*`、`SADDLE_*`、`TERM`、`TERM_PROGRAM`、`SHLVL`、`PWD`、`OLDPWD`、`_`）；这样 agent 也拿到 rc 设好的 PATH（nvm、bun 之类），和普通终端一致。实现时要实测 `--env PATH=…` 能盖过 corral 自己的 PATH、参数长度没问题。
5. **名字**：默认 `<前缀>/<种类>` 加 `--unique`（得到 `paddock/claude-1` 这类）；前缀照 New Agent 的 `suggest_prefix`，取当前目录所在 git 仓库的目录名，不在仓库里取当前目录名。想自己起名：建议用环境变量前缀 `PADDOCK_AGENT=review claude`（替身读走、不传给 agent）；不用命令行参数，免得和 agent 自己的参数撞。标签照 New Agent：`role=regular`，能从命令里读出 model、effort 的（`new_agent::read`）加 `model=`、`effort=`。
6. **一次性命令要放行（坑）**：替身在 PATH 最前，这个 shell 里所有 `claude`、`codex` 调用都会先到它，包括 `claude -p "…"`、`claude --version`、`claude mcp list`、`codex exec …`、`codex login`、脚本里的调用。这些不该变成 corral agent。建议：标准输入或标准输出不是终端、或者是已知的一次性用法（`-p`／`--print`、`--version`、`-v`、`--help`、`-h`，以及第一个参数是子命令的，如 claude 的 `mcp`、`config`、`update`、`doctor`，codex 的 `exec`、`login`、`logout`、`mcp`、`apply`）时，替身直接 `exec` 真程序。
7. **paddock 这边**
   - 窗格仍是 shell（`Shown::Shell`），里面前台跑着 `corral attach`。P5-47（agent 停了自动关窗格）不受影响：agent 结束只是接入结束、回到提示符。
   - P5-66 退出确认会把 `corral attach` 当成“在跑的程序”而问一次；关掉接入不会停 agent，建议把 `corral attach` 不算在内。
   - 左侧栏：agent 会出现（来自 `corral ls`），但 paddock 不知道它正接在哪个窗格里，点卡片会另开一个窗格再接一次。建议替身起完后用 `paddock ctl`（窗格里已有 `PADDOCK_INSTANCE`、`PADDOCK_PANE`）告诉 paddock“这个窗格接着某 agent”，点卡片就聚焦这个窗格；接入结束再告诉一次。
   - 只对从新入口开的 shell 生效：只有那时才用 paddock 的 `ZDOTDIR`／`--rcfile`；普通 shell 不变。
   - 替身做成 `paddock` 程序的一个入口（按程序名分派，替身目录里放指向 `paddock` 的符号链接），Rust 写，不用脚本；corral 程序路径由环境变量传给它。
8. **没测的**：真窗口里接入、Ctrl-]、回到提示符的手感（corral attach 本身 paddock 一直在用）；fish；`--env` 盖 PATH 和长参数。

## 要用户定的

1. 起了 agent 之后窗格怎么样：仍是 shell、前台接着 agent（Ctrl-] 回 shell，agent 结束也回 shell）——推荐；还是把窗格换成 agent 窗格（shell 没了）。
2. 自己起名的方式：`PADDOCK_AGENT=<名字> claude`（推荐），还是别的。
3. 一次性命令放行（第 6 条）——推荐放行。
4. 环境：把 shell 的整个环境传给 agent（推荐，像普通终端），还是只传 `FOO=1` 这种前缀里的。
5. 支持哪些 shell：zsh、bash（都实测过）先做，fish 照同样办法做但没测；别的 shell 不拦。
