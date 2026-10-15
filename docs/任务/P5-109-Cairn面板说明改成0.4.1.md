# 任务：Cairn 面板记录列表的说明改成 cairn 0.4.1 的说法

依据：用户 10-15 经 cairn/main 转达：cairn 0.4.1（cairn P1-11）后台写摘要前先判断这个会话有没有下一个会话用得上的内容，没有的（比如闲聊）不写记录；P5-108 改的 “cairn summarizes each session in the background…” 的 each 不准了，“用户让你改掉。只改措辞，意思是‘cairn 在后台给会话写摘要，没什么可接续的会话会跳过’；别的不用动，命令输出格式都没变。”
执行：主控

## 在哪里干活
- worktree：`../paddock-worktrees/p5-109-cairn-copy`，分支 `p5-109-cairn-copy`（已从 main 建好）。

## 要改的文字（`app/src/cairn_view.rs`，记录列表顶上的说明）
- 现在：`cairn summarizes each session in the background once it has been quiet for 3 minutes or has ended. A new session in this repository starts from the most recent ones.`
- 改成：`cairn summarizes sessions in the background once they have been quiet for 3 minutes or have ended, skipping ones with nothing to carry over. A new session in this repository starts from the most recent summaries.`

没采用页那句（`Once adopted, cairn summarizes the sessions you have here in the background, …`）和 Adopt 确认卡片那段没写 each，泛指“给会话写摘要”，不算说错，照转达不动。

## 怎么算做完
- 用户原话（转达）：“只改措辞，意思是‘cairn 在后台给会话写摘要，没什么可接续的会话会跳过’；别的不用动”。

## 不做
- 不动其他文字、标题、`→ Next session starts from this` 标记和面板的读取、画法。
