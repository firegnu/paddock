# 任务：收件箱回复框能对正在干活的 agent 插话

2026-11-09，paddock/main 写。
类型：功能变更
依据：ranch/main 转来的需求 `../ranch/docs/需求/paddock-回复框能对干活中的agent插话.md`（ranch 提交 `22e5312`）。corral 已做完并部署（R57b，现装 `versions/864d900`）：`corral send <全名> <文字> --steer`，结果和退出码以 ranch `docs/CONTRACT.md`「`corral send NAME TEXT --steer`（R57b）」一节为准。ranch 不用再改。
执行：主控

## 用户原话
- 11-09（对 ranch 说）：「现在委派出去的agent在工作的时候是没有办法send过去东西的，也就是说不能插话。我的理解是这块设计的有问题，我理解的是是可以插话的。」
- 11-09（corral 做完后 ranch 主控说 paddock 回复框还只能回空闲的 agent）：「为什么还有这么多没做的？」「可以，两个都写草稿给我看」
- 11-09（对 paddock 主控）：用户让 ranch/main 把需求直接转给 paddock，照 paddock 自己的流程安排。
- 11-09 看过任务文件和主控另提的两条（“没提交”后先关上回复框、干活时换灰字提示）：「都按你的建议来，开工吧」。

## 先读
- 仓库根的 `CLAUDE.md`。
- ranch 需求原文（上面的路径）和 ranch `docs/CONTRACT.md` 的 R57b 一节。
- `docs/DESIGN.md` §13 P5-87「收件箱」的回复框一条、P5-103。
- 代码：`app/src/inbox.rs` 的 `composer`、`refusal`、`status`、`Sent`；`app/src/window_inbox.rs` 的 `inbox_typing`、`send_inbox`、`open_inbox_pane` 和回复框那段画法；`app/src/corral.rs` 的 `Agent`、`merge`、`Client::send`、`deliver`、`Refused`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-199-inbox-steer`，分支 `p5-199-inbox-steer`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-199-inbox-steer`（从 `.target/main` 克隆）。
- 主要动 `app/src/corral.rs`、`app/src/inbox.rs`、`app/src/window_inbox.rs` 和 `docs/DESIGN.md`。

## 要做的
1. **读能力**：`Agent` 多读 `corral status` 的 `capabilities.steer`（`1` 才算能插话；没有这个字段、不是 `1`、远程机器的旧 corral 都算不能）。本机和远程机器一样处理，不新增轮询。
2. **回复框什么时候能用**：
   - 空闲：和现在一模一样。
   - 在干活（含 Stalled）且能插话：框照常可用，↩ 发送、⇧↩ 换行等都不变；框里的灰字提示从 `Reply to <名字>…` 换成 `Add to its current turn…`（用户 11-09 同意主控的建议）。
   - 在干活但不能插话：和现在一样不给发，说明后面提一句升级后才能插话（英文，例如 `Working… you can reply when this turn ends (upgrade it to reply while it works)`）。
   - 等你点框（blocked）、启动中、暂停、出错等：和现在一样。
3. **发送**：空闲时调 `corral send <全名> <文字>`，不变；在干活时调 `corral send <全名> <文字> --steer`。其余照现在：文字原样一个参数，不加 `--force`、`--after`，不在画界面的线程里等，同一个 agent 一次只发一条（现有的 `sending`）。Kanban Loop、新建 agent 的首句、终端选中送给 agent 这几处的 send 不动。
4. **结果怎么显示**（照需求的表）：
   - 成功（0）：同现在发成功：框清空、留 `You · sent …`；新回复等它这一轮全部做完才有，那一块留到那时（现有 `Sent` 的规则）。
   - 3 `draft_not_submitted`：框清空、字不留（留着会被再发一遍）；框下写「写进去了但没提交，去窗格里看一下」（英文），带 `Open pane`。不显示 `You · sent`。之后这个 agent（同一个实例）的回复框关上，只留这句说明和 `Open pane`，在收件箱里点过它的 `Open pane` 才重新打开：corral 约定里那段草稿会在下一次 send 时合并提交，必须先让人核对（用户 11-09 同意主控的建议）。
   - 3 `not_delivered`：同现在，不重发。
   - 7 `not_idle`、`state_changed`：字留在框里，写原因，可以再发。能插话的 agent 收到 `not_idle`（这边以为它空闲，它刚开始干活），说明写成“它刚开始干活，再发一次就插进这一轮”的意思。
   - 8 `human_active`：同现在。
   - 9 `unsupported_steer`：字留在框里，提示这家还不支持干活时插话，等它空下来再发。
   - 9 `unsupported`：字留在框里，提示等空闲再发。
   - 1 `usage`（干活时以 `/`、`!` 开头，或 PATH 上的 corral 太旧不认 `--steer`）：字留在框里，写原因；不改用 `keys` 硬送。
5. `docs/DESIGN.md`：§13 加 P5-199 一条（用户原话、上面的规则），P5-87 回复框那一条注明“P5-199 改：干活时能插话”；§3／依赖表里 `send` 那句补上 `--steer`。

## 怎么算做完
- 用户原话（验收）：收件箱的回复框，对正在干活的 agent 也能发；发出去的话插进它当前这一轮，它在下一步就能看到，不用等这一轮做完。
- 验证：
  - 先写单测并确认因缺这个行为而失败，再实现。至少覆盖：`composer` 对空闲／干活能插话／干活不能插话／blocked／暂停的结果；发送参数空闲时不带、干活时带 `--steer`；`capabilities.steer` 的读法（有、没有、不是 1）；表里每种返回对应的是“字留在框里”还是“框清空”，以及说明文字。
  - 用假 `corral` 脚本测 `send … --steer` 各退出码（0、3 两种、7、9 两种、1）读成什么。
  - `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 合并安装后留给用户实际试：对一个正在干活的 Claude Code／Codex agent 插一句。主控可以自己开 `paddock/test-<名字>`（`--label role=test`）用 `corral send --steer` 核对命令行为，用完 `corral stop`；不对用户的 agent 插话。

## 不要做
- 不改 Esc、打断这类按钮，不改收件箱别的界面和别的发送入口。
- 只调 PATH 上的 `corral`（远程机器照现有的远程调用）、读它的 JSON，不读 `~/.corral`。
- 失败不退回普通 send、不改用 `corral keys`、不加 `--force`、不自动重发。
- pony（Slack）的插话不归这件。
