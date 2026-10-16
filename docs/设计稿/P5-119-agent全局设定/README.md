# P5-119 Settings 里各 agent 全局设定样稿

用户 10-16 经 cairn/main 转达：“是否可以在paddock的setting中显示出来各个agent的全局设定。而且可以切换到编辑。不管是claude的，还有codex和现在接入的所有的agent。”放哪：“先出样稿再定”。

`instructions.html` 从上往下六屏，直接用浏览器打开（纯 HTML，没有脚本）。配色取 Lagoon 的近似值，实现时从主题取色。文件内容是编的示意，没读真实的全局文件；路径、大小是本机 10-16 的。

| 屏 | 内容 |
| --- | --- |
| 甲 看 | Settings 新开一页 Instructions：顶上六家，下面一张卡（路径、大小、修改时间、Edit），正文照 Markdown 画；底下写明生效时机 |
| 甲 改 | 正文换成等宽多行编辑框，Cancel／Review changes… |
| 甲 对比 | 存之前的前后对比、备份位置、Save；打开后文件被别处改了时的黄色提示（Save 不能点，Reload／Copy my text） |
| 甲 没有文件 | omp：还没有文件、Create file，写明它还读别家的全局文件 |
| 乙 列表 | 放进 Agents 页 Integrations 下面一节，每家一行，Open／Create |
| 乙 弹框 | 点 Open 后的弹框，看、改、对比同甲 |
