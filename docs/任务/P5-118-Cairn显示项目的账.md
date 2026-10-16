# 任务：Cairn 标签显示项目的账

依据：cairn/main 10-16 经用户转达（用户已同意直接做）：cairn 0.5.0 加了项目的长期记忆“项目的账”，请 paddock 面板展示，约定见 cairn DESIGN §7.1，“只加不改”。用户 10-16 定：写的命令（add/edit/close/reopen/delete）面板不做，只读；放在 Cairn 标签哪里“先出样稿再定”；看过样稿：“甲，不加 Show closed，开始做吧”。
执行：主控

## 在哪里干活
- worktree：`../paddock-worktrees/p5-118-cairn-ledger`，分支 `p5-118-cairn-ledger`（已从 main 建好）。

## 约定（cairn DESIGN §7.1，只读、只加不改）
- `cairn ledger --json [--all]`：只读，不收取暂存区；`{"version":N,"items":[…]}`，每条 `id`、`no`、`kind`（state/open/convention，以后可能有新值）、`title`、`body`（可为 null）、`status`、`created_at`、`created_by`（`claude:…`、`codex:…` 或 `user`）、`updated_at`、`closed_at`、`close_reason`、`deleted_at`；默认只有有效条目，按类再按 `no` 排。没有账、旧库、没采用都是 `{"version":0,"items":[]}`。不认识的字段忽略。
- `cairn show --json`（不带 ID）新增 `ledger_item_ids`：注入里账那一节包含的条目 ID；旧版本没有，当空数组。

## 要做的（照样稿甲）
1. `cairn.rs`：采用了的仓库，读的时候加一次 `cairn ledger --json`（不带 `--all`）；解析失败或命令出错按现有的 `Read::Failed` 处理，空账不是错。`ledger_item_ids` 照约定读进来（缺字段当空）。
2. `cairn_view.rs`：照样稿甲，状态行下面加 `Notes | Ledger` 切换（记住上次选的），Ledger 页每组一个凸起框（`raised.rs`）；条目按 kind 分组（State／Open／Conventions，不认识的 kind 归到最后一组 Other），每条 `#no` 和标题，点开看正文（Markdown）和 Added／Updated／By／Entry；空账显示 `No ledger entries yet`。界面文字英文。
3. 只读：不调用任何写账的命令，不加按钮。
4. 测试用假 `cairn` 脚本和合成数据：有条目、空账、旧版本（没有 `ledger` 子命令／没有 `ledger_item_ids`）、不认识的 kind 和字段。

## 验证
- `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
- 不起窗口截图（主控不在 agent 会话里起测试窗口），样子留给用户实际看。

## 怎么算做完
- cairn/main 转达的要求：(1) 用 `cairn ledger --json` 展示账；(2) 读 `show --json` 的 `ledger_item_ids`，旧版本缺字段当空数组；(3) 空账显示成“还没有条目”这类状态，不要当出错；(4) 写的命令要不要做问用户——用户定只读。

## 不做
- 写账的命令（用户 10-16 定只读）。
- `--all` 看已了结、已删除的条目（用户 10-16：“不加 Show closed”）。
