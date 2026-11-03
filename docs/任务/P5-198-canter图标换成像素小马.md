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

## 完成记录
- 做了什么：仓库剪影 `app/assets/kinds/canter.svg` 换成小马轮廓：照像素图每行连续的不透明像素拼成一条路径，放在 16×16 画布的第 4～11 行，`shape-rendering="crispEdges"`。用户本机放了彩色版 `~/.config/paddock/icons/canter.svg`：每段同色像素一个方块，颜色照 canter 的 `pony()`，16×8，同样不平滑；`SOURCES.txt` 记了一行。两份都用 awk 从同一份像素图生成，不手抄坐标。README 的 canter 一行和那段说明、DESIGN 的 P5-195 图标一条改成小马，写明出处和 Apache-2.0。品牌色没动。
- 验证了什么：`cargo test --all-targets` 1111 个通过，`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。界面上的样子留给用户重开 paddock 看。
- 拿主意的地方：剪影把眼睛、高光也填上，只留轮廓（任务写的是不透明像素全填）。
- 没做的事：没有截图验看。本机有彩色版，所以这台机器上看不到仓库的剪影，只有没放真图标的机器才会显示它。
- 踩过的坑：没有。

## 返工 1：换成平滑的可爱小马
- 用户看了像素版（截图里侧栏的 canter 卡片）：「能不能整一个可爱的小马，现在这个像素级别的和其他的很不协调啊。」别家的真图标是满格的方形应用图标，paddock 裁成圆角；16×8 像素放大后全是锯齿。
- 主控画了一匹平滑的小马头（侧脸朝右：栗色头和脖子、深棕鬃毛和刘海、奶油色鼻子、大眼睛带高光、腮红、笑嘴），颜色取 canter `pony()`；满格底色出了奶油、浅绿两版，用 `rsvg-convert` 渲染成对比图给用户看。用户选「浅绿底」。
- 本机 `~/.config/paddock/icons/canter.svg` 换成浅绿底这版，`SOURCES.txt` 那一行改了。仓库剪影换成同一匹小马的轮廓（头、耳朵、鬃毛、脖子），眼睛挖空；README 记为 paddock 自己画的，DESIGN 那一条改了。像素版不再用。
- 验证：两张都用 `rsvg-convert` 渲染成 256 px 看过；`kind_icon` 的测试照旧通过。界面上的样子留给用户在 paddock 里看。
- 用户随后：「不好意思，换成奶油色的吧。」本机图标换成奶油色底那版，`SOURCES.txt` 跟着改；仓库剪影不受影响。
- 用户再改：「底色颜色换成绿色，但是要深一些。」主控出中绿、较深、深绿三档对比图，用户选「较深」（`#7DB466` 到 `#5C9A4A`）。本机图标和 `SOURCES.txt` 换成这版。
