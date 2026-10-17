# 任务：sandbox 在 paddock 里看得见、能选（界面）

依据：用户 10-16：“再加一个draft task。委派的agent放到sandbox中，要设计一套sandbox”。10-17 讨论后定下墙本身放在 ranch（corral 开 agent 时可选套上），讨论全文和已定的事搬到 ranch 的 `docs/任务/R17-agent放进sandbox.md`；用户：“对，搬到ranch。这边只留界面的东西”。本件只做 paddock 的界面，要等 ranch 的 R17 做出公开命令（`corral start --sandbox`、`corral status` 里的档案字段、`ranch` 下的 sandbox 命令）再动手。范围没定。

## 设想的界面（主控 10-17 提的，用户还没定要哪几处）
- New Agent 对话框里选用哪个档案，不选就是现在的样子（用户：“加了sandbox就是sandbox里面跑，不加就是现在的形式”）。
- 卡片上一个小锁，表示这个 agent 在墙里，悬停写档案名。
- 一个面板列出它被拦下的操作（例如“想写 `~/.ssh`，被拦了”），从 R17 的 `ranch` 命令读。
- Settings 里看和改档案。

## 还没定
- 上面几处要不要、先做哪个；被拦记录面板放在哪（卡片详情、右侧栏、还是单独一页）。
- 远程机器（5090）的 agent 要不要显示，等 R17 做 Linux 再说。
- 出样稿给用户看过再定。

## 规矩
- paddock 只调 ranch 的公开命令，不读 sandbox 的内部文件（CLAUDE.md「与 Saddle 分开」「Corral 只走公开命令」）。

## 怎么算做完
- 用户原话：“这边只留界面的东西”。（范围定了再补。）
