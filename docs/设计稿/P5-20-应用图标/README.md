# P5-20 应用图标样稿

用户 10-06 定：“纯黑，自己找的素材，照现在这样做”（见 `docs/DESIGN.md` §13 P5-20）。原稿在 claude.ai 私有画布「P5-20 paddock 应用图标」页，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。

- `Icon.dc.html`：图标本体，1024 坐标系的 SVG；末尾 `renderVals()` 里 `head` 是猫头路径（100×100 坐标，`translate(197 226) scale(6.3)` 放进 1024），眼睛是 evenodd 挖空；参数 `split`（色散宽度，默认 1，单位同猫头坐标）、`glow`（光晕）、`ground`（采用 `black`）。
- `Main.dc.html`：各尺寸和程序坞里的样子；32 及以下去掉色散和光晕。
