# 任务：paddock 技能正文（告诉 agent 什么时候、怎么用 paddock ctl）

2026-10-07，paddock/main 自己写。
类型：样式／文案调整
依据：本轮只改写 `app/resources/skills/paddock/SKILL.md` 的正文（P5-39a 放的是占位），照 Saddle `skills/saddle/SKILL.md` 的约定改成 paddock 版，命令和参数以 P5-39a 的 `paddock ctl --help` 为准；安装和卸载 saddle 技能由用户在终端里做。
依赖：P5-39a
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-39c-skill`，分支 `p5-39c-skill`（已从 main 建好）。

## 要做的
用户 10-07：“配，saddle 技能也卸掉，按你的方案派出去做吧”。技能写清：什么时候用（只在用户要求操作 paddock 界面时）、先 inspect 认清自己、open／browse／close／request 的用法和结果怎么读、关闭含 shell 要先问用户、busy 怎么办、同一 request ID 重试、不能做的事（发键、读屏、停 agent、读网页；停 agent 走公开 corral 并要用户授权）。开头的归属标记保持不变。

## 怎么算做完
- 正文和 `paddock ctl --help` 一致；`git diff --check`；`app/` 下跑技能相关测试和 `cargo test --all-targets`、clippy、fmt。P5-39b 合并后对着实际行为再核一遍。
