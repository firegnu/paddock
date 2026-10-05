# P5-12 调研：agent 种类的官方 logo 与使用条件

2026-10-06，主控派只读调研 agent 查的，结论只依据下面列出的来源；查不到的标“未知”。还没给用户定。

## Claude Code（Anthropic）
- 官方素材：Anthropic press kit（`https://www.anthropic.com/press-kit` 跳到一个约 26MB 的 zip，最后修改 2026-02-06）。`Claude logos/` 里有：Claude 文字标；Claude Code 文字标（很宽，不适合 14–16pt）；`Claude Spark - Clay.svg`（94×94 小符号，单色 `#D97757`，最适合小尺寸）；128×128 的 App 图标（带底色和渐变，缩小会糊）。
- 没有 Claude Code 专用的小图标。code.claude.com 的 favicon 是像素小怪兽，只有 PNG；Simple Icons 有一份照它重画的 `claudecode.svg`，不是 Anthropic 发布的。
- 条款：Trademark Guidelines（2024-08-01 生效，`https://www.anthropic.com/legal/trademark-guidelines`）：只能按许可使用、材料事先批准；不得改色、字体、比例；不得暗示赞助、背书或合作；四周留白；不加 ™。没有“指称性使用免批准”的条款，没提开源或第三方集成。许可申请：marketing@anthropic.com。
- 信号：按字面，把 SVG 放进公开仓库属于未获许可。

## Codex（OpenAI）
- 官方素材：`https://openai.com/brand/` 没提 Codex，也没有 SVG/zip 下载；developers.openai.com/codex 只有 favicon.png。没找到官方发布的 Codex 专用标志。`openai/codex` 仓库（Apache-2.0）里没有 logo 文件，Apache-2.0 不授予商标权。
- 品牌页条款：只在直接关联 OpenAI 服务时使用；按原样使用、不得修改；未经许可不得使用；不暗示背书；不比自己的标更显眼；Blossom 不加颜色、不作主品牌；许可非独占、不可转让，可随时撤回。许可申请：partnercomms@openai.com。
- 信号：Simple Icons 16.0.0 删掉了 OpenAI 图标（PR simple-icons#13944，原因是拿不到许可）；公开仓库附带 SVG 等于转给别人用，和“不可转让”冲突。

## pi（`@earendil-works/pi-coding-agent`）
- 官方素材：`https://pi.dev/press-kit`：主 logo `https://pi.dev/logo.svg`（800×800 三色）；Badge `https://pi.dev/favicon.svg`（560×560 单色，深浅色自动切换，官方说明 “Square mark for favicons and compact badges”，适合 14–16pt）。
- 许可：仓库 MIT，但 SVG 不在仓库里；网站页脚写 “MIT License”，是否覆盖 logo 不明确；没有商标政策页。Simple Icons 收录了 Pi。

## omp（oh-my-pi，`can1357/oh-my-pi`）
- 官方素材：网站 favicon `https://omp.sh/favicon.svg`（64×64，深色圆角方底上渐变 π）；仓库 `assets/icon.svg`（120×90，白色 π 加橙色插头），两者不是同一个图形。
- 许可：仓库 MIT（保留版权声明），`assets/icon.svg` 在仓库里，按字面 MIT 覆盖；网站 favicon 许可未知；没有品牌或商标页。favicon 自带深色底，放深色背景上会多一个方框。

## 小结
| 种类 | 小尺寸官方标志 | 能否放进公开仓库 |
| --- | --- | --- |
| Claude Code | Claude Spark（Claude 的，不是 Claude Code 专用） | 按条款需事先批准 |
| Codex | 没找到 | 需许可，且许可不可转让 |
| pi | 有（favicon badge） | 不明确，大概率可以 |
| omp | 有（仓库 icon，MIT） | 仓库那份可以 |
