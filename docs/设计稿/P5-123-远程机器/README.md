# P5-123 远程机器藏起来、放回来、删掉 样稿

用户 10-16：“我想能够配置显示不显示某一个remote。”“提供可以删除某一个remote的入口”；入口选了 Settings 加一页再加右键，藏起来当成没配。

原稿在 claude.ai 私有画布「P5-123 远程机器样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里；画布上的 `Main` 即 `A-Settings`）。配色取 Lagoon 近似值，实现时从主题取色；机器名、ssh 目标是编的。

| 文件 | 内容 |
| --- | --- |
| `A-Settings.dc.html` | 甲：Settings 新开 Machines 一页，每台一行（状态点、名字、ssh 目标、状态、Show 开关、Remove…） |
| `B-Remove.dc.html` | 乙：Remove… 的确认，写明不碰那台机器上的 agent，可改成 Hide instead |
| `C-Menu.dc.html` | 丙：切换栏胶囊上右键：Rename…、Hide、Remove…、Machines Settings… |
| `D-Hidden.dc.html` | 丁：藏起一台后的切换栏；全藏起来时切换栏不画 |
| `E-Rename.dc.html` | 戊：改名（用户加的）：name 当身份不动，另加显示名 label，Settings 一行点名字就地改；胶囊、窗格小签显示它 |
