# paddock ctl 方案

2026-10-07，paddock/main。用户 10-07：“paddock ctl我觉得可以做”，要主控先给方案。已定的部分见 `docs/DESIGN.md` §3 第 5 步（用户 10-05：“这个也要迁移”）；本文补上要用户定的几处。依据：Saddle `src/control.rs`（413 行）、`src/control_cli.rs`（238 行）、`src/app_control.rs`（593 行）和 Saddle 的 `saddle` 技能（只读）。

## 1. 做什么

让 agent 和脚本在**正在运行的 paddock 窗口里**动手：开 shell、显示或新开 agent、按标签页和上下左右分屏摆放、关闭显示，并查询结果。典型用法：主控派活后，用一句命令把新 agent 摆到自己右边；agent 起了开发服务器，把网址在右侧栏 Browser 里打开给你看。

## 2. 命令（和 `saddle ctl` 一致，加一条 Browser）

| 命令 | 作用 |
| --- | --- |
| `paddock ctl inspect` | 当前实例的标签页、窗格、布局，以及“调用者自己在哪个窗格” |
| `paddock ctl instances` | 列出运行中的 paddock 实例（一般只有一个） |
| `paddock ctl open --place tab\|left\|right\|up\|down …` | 三选一：`--shell [--cwd DIR]`、`--agent NAME`（已有的 agent；已在别处显示就挪过来）、`--name NAME [--cwd DIR] [--role …] -- PROGRAM ARG…`（新开，经公开的 `corral start`）。默认相对调用者自己的窗格；默认不抢焦点，`--focus` 才切过去 |
| `paddock ctl close --pane ID \| --tab ID` | 关闭显示（只断开 paddock 的接入，不停 agent）；里面有在跑的 shell 时先返回要确认的清单和凭据，用户确认后带凭据再关 |
| `paddock ctl request ID` | 查一次修改的结果：accepted、starting、complete、failed…（`complete` 只表示显示好了，不表示模型就绪） |
| **新增** `paddock ctl browse URL [--focus]` | 在右侧栏 Browser 打开网址（只认 `http`／`https`，本地地址照 P5-28c 的规则补全）；右侧栏没开就打开它 |

所有输出都是 JSON；每次修改返回 `instance`、`request_id`、`pane`、`revision`，可预先给 `--request-id` 以便超时后查原请求、不重复执行。

## 3. 怎么实现

- **传输层和命令行照搬 Saddle**（DESIGN §3 第 5 步已定）：每个实例一个私有 Unix 套接字（目录 0700、套接字 0600，只有你本人的进程能连），JSON 消息，有长度、超时和队列上限；最多记 256 次修改。目录用 paddock 自己的（`$XDG_RUNTIME_DIR/paddock`，没有时退到用户私有临时目录），不碰 Saddle 的。迁入的文件开头注明“From Saddle …”。
- **界面端对着 paddock 重写**：窗格／标签页／分屏用 paddock 的 `layout.rs`；新开 agent 复用 New Agent 的“开好再放到指定位置”；`browse` 复用 Browser 的打开网址。
- **认出“调用者是谁”**：corral 开的 agent 用 corral 已经设置的 `CORRAL_NAME`、`CORRAL_INSTANCE`；paddock 里开的普通 shell，由 paddock 注入 `PADDOCK_INSTANCE`、`PADDOCK_PANE`。认不出时，依赖“自己的位置”的操作直接报错，不猜。
- **命令放在哪**：`paddock ctl` 是 paddock 程序本身的一个子命令（不起窗口，只发一条消息就退出）。agent 要能在命令行里直接敲 `paddock`，需要在 `~/.local/bin/paddock` 放一个指向 `~/Applications/paddock.app` 里程序的链接——这是改全局位置，由你在终端里跑一次（主控没有权限，见 HANDOFF“悬着”第一条）。
- **配套技能**：照 Saddle 的 `saddle` 技能改写一份 `paddock` 技能（什么时候用、怎么查结果、关闭前要确认），装到 `~/.claude/skills/paddock`、`~/.agents/skills/paddock`，同样由你在终端里装一次。

## 4. 安全边界（和 `saddle ctl` 一样严）

- 能做的只有：开、摆放、关闭**显示**、查询、在 Browser 打开网址。
- **不能**：往任何终端发按键、读屏幕内容、停止 agent（停 agent 仍走公开的 `corral stop`，按你的授权）、运行脚本、读 Browser 的网页内容。
- 关闭里面有在跑的 shell 的窗格／标签页：一定先返回影响清单，要你确认后才关。
- 你正在拖分隔线、开着对话框或确认框时，返回 `busy`，等你操作完再试，不打断你。

## 5. 怎么分活

1. **P5-39a 传输层和命令行**（照搬，Codex 常规）：套接字、消息、`instances`／`request`、调用者身份、命令行解析；带测试。
2. **P5-39b 界面端**（Claude Code 常规；依赖 a）：`inspect`、`open`、`close`（含确认）、`browse`，接到窗口和布局上；带测试。
3. **P5-39c 技能和安装说明**（主控自己写；依赖 b）：技能文本；你在终端里建 `~/.local/bin/paddock` 链接、装技能。
- 套接字和“关闭前确认”碰到安全，a、b 合并前各交一次 Codex 交叉审查。

## 6. 要用户定的

1. **范围**：和 `saddle ctl` 一样，再加 `browse`（在右侧栏 Browser 打开网址）？主控建议加：agent 起了服务就能直接摆给你看。
2. **命令放在哪**：同意用 `~/.local/bin/paddock` 链接到 app 里的程序（你跑一次）？
3. **技能**：要不要配一份 `paddock` 技能，让 agent 知道什么时候、怎么用它（你装一次）？主控建议要；没有技能 agent 不会主动用。
