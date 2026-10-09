# P5-58 应用图标样稿

用户 10-09 定：“1: 甲 2: 马 3: 可爱多彩”，再定“Candy，圆鬃毛”（见 `docs/DESIGN.md` §13 P5-58）。原稿在 claude.ai 私有画布「P5-58 paddock 应用图标」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。

- `Main.dc.html`：第一轮，马、高地牛、公羊 × 甲（可爱多彩）、乙（极简，沿用 P5-20 的黑底白剪影加色散），带 128、64、32、16 的缩小图；甲的配色可在 Tweaks 里切换。
- `Horse.dc.html`：第二轮，马 × Lagoon、Sunset、Candy、Aurora 四组配色 × 尖、圆两种鬃毛。
- `Final.dc.html`：定稿（Candy、圆鬃毛，加了额毛），各尺寸、32 和 16 的简化版与不简化的对比、深浅桌面上的程序坞。实现照它做，SVG 在 `app/src/icon/paddock.svg`、`paddock-small.svg`。
