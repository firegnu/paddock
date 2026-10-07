# P5-37 New Agent 重新设计样稿

用户 10-07：“new agent那个对话框再设计一下，现在的太初级了。”主控出三张样稿，推荐“B 为主体＋C 的预设”，用户：“可以。同意你的建议。”原稿在 claude.ai 私有画布「New Agent 重新设计」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。配色取自 Dune；项目、模型名是示意。

| 文件 | 内容 |
| --- | --- |
| `B-Composer.dc.html` | B 先写任务：大输入框＋一排可点开的选项（项目、种类、模型、强度、打开位置、角色），⌘↩ 创建（**采用为主体**） |
| `C-Presets.dc.html` | C 预设：左侧常用组合（Controller、Developer、Codex builder、Quick look…），可存当前设置为预设（**采用其预设，做成 B 输入框上方的一排按钮**） |
| `A-Form.dc.html` | A 精致表单（未采用；其中“预览开出来的卡片”不采用，命令收进 Show command 采用） |
