# 任务：调研“开一个普通 shell，里面敲 claude／codex 就由 corral 托管”怎么做

2026-10-09，paddock/main 写，主控自己做（要读用户的 shell 配置，结论里不写配置内容，免得私人数据进公开仓库）。
类型：调研
依据：
- 用户 10-08：“一般来说herdr中就是新建agent的时候，就是弹出一个terminal,然后用户在terminal中输入codex --yolo， claude，pi之类的。直接就跑起来了。我想增加这么一个上面我描述的新建agent的流程，现在的新增agent的保持不变。”“对，要的是corral托管，而不是那种直接从命令行启动的agent。”
- 用户 10-09 选：真 shell（不是一行命令提示）；入口放新建标签／分屏面板里一行；名字自动起、能改。补充：“我觉得就是普通的shell，就像herdr那种的。new agent之后就是开一个普通的shell，然后这个shell就是正常的terminal，用户可以当成正常的terminal使用，然后可以codex，claude等命令直接启动，但是这个后台你得默认让corral托管。”其他 shell：“同上”。同意先调研。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-61r-agent-shell`，分支 `p5-61r-agent-shell`（已从 main 建好）。
- 只写 `docs/调研/P5-61r-agent-shell.md` 和本文件末尾的完成记录；原型放 scratchpad，不进仓库。

## 要查清的
1. 拦法：在这个窗格的 shell 里让 `claude`、`codex`、`pi`、`omp` 先找到 paddock 的同名小程序（PATH 最前面一个目录），还是别的办法（shell 函数、钩子）。用户的 rc 文件会改 PATH：小程序目录怎么保证排在最前、对 zsh／bash／fish 各要怎么做。
2. 小程序怎么找到“真正的”程序：跳过自己的目录，在 PATH 里找下一个同名的（用户的 `codex` 本身是一层包装，要保留）。
3. 交给 corral：`corral start <名字> --cwd <当前目录> [--env …] -- <真程序> <参数>`，再在同一窗格 `corral attach`；接入退出后回到 shell 提示符、agent 照样在 corral 里跑。corral 的状态钩子（看程序名）能不能照常装上。
4. shell 自己处理的东西照常：别名、变量、引号、`FOO=1 claude` 前缀（环境变量怎么带进 corral：corral 起的 agent 默认拿到的是什么环境）。
5. 名字：自动起 `<仓库>/<种类>-N`、不撞名；用户想改名字怎么给（参数、环境变量，或提示里改）。
6. paddock 要知道什么：窗格里接上了哪个 agent（标题、左侧栏对上、P5-47 agent 停了窗格怎么办、P5-66 退出确认把 `corral attach` 当成在跑的程序）。
7. 只对从这个入口开的 shell 生效，普通 shell 不受影响。

## 不做
- 不起 paddock 窗口（10-09 事故，见 P5-52）。
- 不改用户的 rc 文件和任何配置；原型只在临时目录、临时 HOME 或子 shell 里试。
- 不对用户的 agent 做任何事；实测要起 agent 时自己开 `paddock/test-<名字>`（`--label role=test`），用完按规矩 `corral stop`。

## 怎么算做完
- 调研文档写清：每一问的事实（实测过的标明怎么测）、推荐做法和取舍、还拿不准的；给出要用户定的问题。

## 完成记录

- 做了什么：调研文档 `docs/调研/P5-61r-agent-shell.md`。结论：在这个窗格的 shell 里把 paddock 的替身放在 PATH 最前（zsh 用 paddock 的 `ZDOTDIR` 加 `precmd`／`preexec` 钩子，bash 用 `--rcfile` 加 `PROMPT_COMMAND`），替身用 `corral start` 起真程序（绝对路径）、再前台 `corral attach`。
- 验证了什么：临时交互 zsh（用户真实配置）里普通命令、别名、`FOO=1` 前缀都到了替身；bash 同样；真 `claude` 用绝对路径交给 corral 后 `kind: claude`、钩子照常（`SessionStart` → `idle`）；corral 起的 agent 不继承调用方的环境和 PATH，`--env` 能传过去。测试实例 `paddock/test-env-1/2`、`paddock/test-shim-1` 都已停，`corral ls` 里没有残留。
- 拿主意的地方：用户配置只看“改不改 PATH、有没有给 agent 起别名”的行数，内容不进文档；原型放 scratchpad，不进仓库。
- 没做的事：没起 paddock 窗口；fish 没装没测；`--env` 盖 PATH 和长参数留给实现时实测。要用户定的五件写在文档末尾。
