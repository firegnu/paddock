---
name: paddock
description: 在正在运行的 paddock 窗口里开 shell、显示或新开 corral agent、按标签页和上下左右分屏摆放、关闭显示、在右侧栏 Browser 打开网址。只在用户要求操作 paddock 界面时用；不用于派发开发任务。
---
<!-- paddock-skill: 1; 由 paddock install-skills 写入 -->

用公开的 `paddock ctl`，所有输出都是 JSON。不要另开第二个 paddock，不读 paddock 或 corral 的内部文件。准确语法以 `paddock ctl --help` 为准。

## 什么时候用

只在用户明确要求操作 paddock 界面时用，例如“把它开在我右边”“开个 shell 到新标签”“把这个网址在 Browser 里打开给我看”。用户只说“开一个 agent”时，按 corral 技能开，不必摆到界面上；不要自作主张摆窗口、抢焦点。

## 先认清自己

先运行 `paddock ctl inspect`，读 `instance`、标签页和窗格，以及 `caller.pane`（你自己所在的窗格）。

- corral 开的 agent 靠 `CORRAL_NAME` 和 `CORRAL_INSTANCE` 认出自己；paddock 里开的 shell 靠 paddock 注入的 `PADDOCK_INSTANCE`、`PADDOCK_PANE`。
- 认不出自己（`caller.pane` 为空、报错或不明确）时，停下依赖“自己位置”的操作，如实说明；不要按名字猜，也不要把用户当前的焦点当成自己。
- 有多个实例时运行 `paddock ctl instances`，用返回的实际 ID，并问用户用哪个；不要挑“最新”的。

## 开和摆放

```sh
paddock ctl open --place right --shell
paddock ctl open --place tab --shell --cwd /absolute/project
paddock ctl open --place down --agent project/review
paddock ctl open --place right --name project/helper --cwd /absolute/project -- codex --yolo
```

- `--place` 只有 `tab|left|right|up|down`，默认相对你自己的窗格；`--relative-to active` 相对用户当前的窗格，也可以给一个窗格 ID。
- 三种内容只能选一种：`--shell [--cwd]`、`--agent NAME`（已有的 agent；已在别处显示就挪过来，不重复接入）、`--name NAME [--cwd] [--role regular|controller] -- PROGRAM ARG…`（经公开的 `corral start` 新开）。参数原样传给 corral，不会自动加权限、模型或强度。
- `--prompt TEXT` 只在用户明确给了第一句话时才带。用户只要求开 agent，不等于授权给它派任务。
- 默认不抢焦点；只有用户明确要求切过去时才加 `--focus`。

## 在 Browser 打开网址

```sh
paddock ctl browse http://localhost:5173
```

只收 `http`／`https`；本地地址可省略协议。右侧栏没开会打开并切到 Browser。默认不把键盘交给网页，用户要求时才加 `--focus`。

## 读结果

每次修改返回 `instance`、`request_id`，以及相关的 `pane`。随后查询：

```sh
paddock ctl request REQUEST --instance INSTANCE
```

- `accepted` 只表示收到；`starting`、`attaching` 表示还在进行（新开 agent 时先 `corral start`，此时 `pane` 还是空的，接入后才有）；`complete` 表示显示好了，**不代表模型已经就绪**。
- `failed`、`target_invalid`、`uncertain` 必须如实报告。目标关了，新开的 agent 仍可能已经存在：继续查原请求，不要自动停掉或重建。
- `busy` 表示用户正在操作（拖分隔线、开着确认框、弹出菜单或面板、New Agent 正在创建，或 paddock 正在退出），此时什么都没改：等用户操作完再试，不要催、不要反复刷。
- 超时或拿不到回执时，先用原 `request_id` 查询；需要重试时用**同一个** `--request-id`、相同参数，绝不换新 ID 重发。同一 ID 换了参数会报冲突。实例最多记 256 次修改，满了会拒绝新修改。

## 关闭显示

```sh
paddock ctl close --pane PANE --instance INSTANCE
paddock ctl close --tab TAB --instance INSTANCE
```

- 关闭只断开 paddock 的显示，**不停 agent**。
- 目标里有在跑的 shell 时，会返回 `confirmation_required`、受影响的清单和一次性的 `confirmation`，此时什么都没关。把清单里的 shell、目录和前台任务会结束这件事告诉用户；用户同意这些具体影响后，用原目标、原凭据和一个**新的**请求 ID 再来：

```sh
paddock ctl close --tab TAB --instance INSTANCE --confirmation TOKEN --confirm-shells --request-id NEW_ID
```

  凭据因为目标变化失效时，重新查询影响，不能绕过确认。

## 不能做的事

- `paddock ctl` 没有发按键、读屏幕、读网页内容、停 agent、跑脚本的命令，也不要想办法绕过。
- 不要把“关闭”理解成停止 agent。用户明确要求停 agent 时，先用公开的 `corral status NAME` 核对身份，再按用户授权调用 `corral stop NAME`。
- 只操作用户要求的那些窗格；不碰用户正在用的其他窗格和 agent。
