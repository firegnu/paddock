# 任务：换了打字对象之后的第一个字一定亮（修 P5-77 的名字签该亮不亮）

2026-10-10，paddock/main 写，主控自己做（规则在 `to_tag.rs`，记录的地方在 `view.rs`、`window.rs`，上下文在手）。
类型：Bug 修复
依据：
- 用户 10-10 用过 P5-77 后：“感觉有时候该亮的时候没有亮。”“现在paddock倒是还挺好用的，但是隔壁的codex-1一直都不行。其他的两个claude时好时坏。就是这种问题通常发生在切换agent之后。”
- 主控查到原因：用户的窗口只有一个标签、一个窗格，在侧栏点别的 agent 是同一个窗格换着显示（`Placement::Pane`，同一个 `TerminalView` 重新 attach）；“上次打字的时间”按窗格记，所以在 agent1 打完字 10 秒内换成 agent2 再打字不亮。P5-77 任务文件里“每个窗格各算各的”是主控写的，没考虑到一个窗格会换着显示不同的 agent。
- 主控提了改法：换了打字对象之后的第一个字一定亮（同一个窗格换成另一个 agent、切标签、分屏里换焦点），没换对象时照旧停满 10 秒后的第一个字亮；“每按一次回车下一条也亮”这次不加。用户：“可以，开始做吧”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-77b-rearm`，分支 `p5-77b-rearm`（已从 main 建好）。

## 要做的
1. 换了打字对象之后的第一个字一定亮，不管这个窗格隔了多久没打字。换对象指：同一个窗格换成显示另一个 agent；当前窗格换成另一个（切标签、分屏里换焦点、关掉窗格后焦点移到相邻的）。
2. 没换对象时照旧：这个窗格停满 10 秒没打字后的第一个字亮。亮多久、怎么淡回、放在哪、写什么都不变。
3. DESIGN §13 的 P5-77 条里“什么时候亮”照这个改。

## 不做
- 不加“每按一次回车，下一条的第一个字也亮”。
- 查找条被盖住、窄窗格名字被裁那两处建议改不在这次。

## 怎么算做完
- 用户原话：“感觉有时候该亮的时候没有亮”“这种问题通常发生在切换agent之后”。
- 先写一条测试并看它因为这个缺陷失败：在一个对象上打字，3 秒后换了对象再打字，要亮；接着打不再亮。再改到通过。`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过。
- 样子留给用户看：重点是切到 Codex 后第一个字亮不亮（主控只能解释到“多半是刚在别处打完字就切过去”，修完要用户再看）。

## 完成记录

- 做了什么：`to_tag.rs` 加 `Typing::rearm`（清掉“上次打字的时间”，下一段文字必亮）和 `Target`（记“当前窗格＋它显示的 agent”，`moved` 回答和上次比变没变）。`window.rs` 的 `focus_active`（所有换标签、换焦点、换显示的 agent、关窗格之后都走它）在把键盘交给当前窗格时比一下对象，变了就让那个窗格的视图 `rearm_tag`（`view.rs` 新加，一行转发）。DESIGN §13 的 P5-77 条“什么时候亮”补上“或者换了打字对象”，另加 P5-77b 一条。
- 验证了什么：先写两条测试并看它们因为这个缺陷失败——`the_first_text_after_the_typing_goes_elsewhere_lights_the_tag_whatever_the_pause`（打字 3 秒后换对象再打字，断言要亮，实际不亮）、`the_typing_goes_elsewhere_with_another_pane_or_another_agent_in_the_same_pane`（同一个窗格换 agent 要算换了对象）；实现后通过。`app/` 下 `cargo test --all-targets`（526 项全过：原 524 加新增 2）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过。
- 拿主意的地方：比对放在 `focus_active` 而不是每个改布局的地方各写一遍（它是这些路径共用的收尾）；不改对象的调用（关弹框后把键盘还给窗格等）不会重新点亮。当前窗格换成 shell 或空窗格再换回 agent 也算换了对象。`rearm` 只清“上次打字的时间”，正亮着的不受影响。
- 没做的事：没起 paddock 窗口，切换后亮不亮留给用户看。窗口这一段（`focus_active` 里的三行）没有自动测试，只测了它用的两条规则。Codex “一直都不行”只解释到“刚在别处打完字就切过去”；如果停很久再去 Codex 打字仍不亮，另有原因，要再查。没加回车触发；查找条被盖、窄窗格名字被裁没动。
