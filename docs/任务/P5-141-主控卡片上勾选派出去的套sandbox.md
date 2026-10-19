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
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-141-child-sandbox`，分支 `p5-141-child-sandbox`（开工时从 main 建）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-141-child-sandbox`。

## 规矩
- paddock 只调 `corral`、`fence` 公开命令；不读 corral、fence 的内部文件；不改 ranch。
- 不对用户的主控调 `corral child-sandbox` 设值（查可以）；实测用自己开的测试主控 `paddock/test-<名字>`（它得是 `--label role=controller` 才有这一格，所以这次不用 `role=test`，靠名字认），它派出去的也只用 `paddock/test-*`，用完都 `corral stop`。
- 界面文字英文。
