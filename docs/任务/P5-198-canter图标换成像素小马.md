# 任务：canter 的图标换成它欢迎头那匹像素小马

2026-11-03，paddock/main 自己做。
路由：轻档自做，不问路由（主控判断：只换一张剪影、本机放一张彩色图标、改几处说明，改动点能一条条列出来）
类型：样式／文案调整
依据：联调后用户说 canter 的图标不对；canter/main 答 canter 没有图标文件，唯一的形象是终端欢迎头的像素小马。本轮只换图标，品牌色不动。
执行：主控
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。

## 用户原话
- 11-03 联调后：「有一个小问题，现在图标不对啊。你问一下canter的主控。」
- 主控问换不换成小马、换哪几处：用户选「彩色小马 + 单色剪影」。
- 品牌色：用户先选改成栗色；主控随后查到栗色和各主题 claude 的橙色几乎一样（Tide Light 的 claude 是 `#C0704A`），重新问，用户选「保持绿色」。

## 查到的
- canter/main 的回答：canter 仓库没有 svg、png 这类图标或 logo 文件；欢迎头左边的像素小马是 P2-47 时用户选的方案 A，是 canter 唯一的形象。出处 canter `crates/canter-tui/src/welcome.rs` 的 `PIXELS`，颜色在 `crates/canter-tui/src/theme.rs` 的 `pony()`；canter 是 Apache-2.0，公开在 `github.com/firegnu/canter`。
- 像素 16×8（`.` 透明）：

```
..........b..b..
........mmbbbbb.
.......mmbwebbbb
.......mbbeebbbb
.mm....mbbbbbpcc
m..bbbbbbbbbbcc.
...bbbbbbbbb....
...m.m...m.m....
```

  颜色：`b` 身体 `#C0703F`，`m` 鬃毛、尾巴、蹄子 `#5B3220`，`e` 眼睛 `#1E1A18`，`w` 高光 `#FFFFFF`，`c` 鼻子 `#F1D2A6`，`p` 腮红 `#E89A8C`。

## 在哪里干活
- worktree：`../paddock-worktrees/p5-198-canter-pony`，分支 `p5-198-canter-pony`（已从 main 建好）。

## 要做的
1. 仓库的单色剪影 `app/assets/kinds/canter.svg` 从马蹄换成这匹小马的轮廓：所有不透明的像素都填上，方格不做平滑。
2. 本机彩色图标：照上面的像素和颜色做 `~/.config/paddock/icons/canter.svg`，方格不做平滑；不进仓库，同目录 `SOURCES.txt` 记一行来源。
3. `app/assets/kinds/README.md` 的 canter 一行、`docs/DESIGN.md` 的 P5-195 一条改成小马，写明出处和许可。
4. 品牌色（各主题的 `canter` 绿色）不动。

## 怎么算做完
- 用户在 paddock 里看到 canter 的图标是小马。
- `kind_icon` 的测试照旧通过；合并前照项目规矩跑全套检查。
