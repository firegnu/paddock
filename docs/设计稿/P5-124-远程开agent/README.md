# P5-124 在远程机器上开 agent（简单版）样稿

用户 10-16：“remote也可以启动agent（但是功能比较简单的那种，就是能启动一个就行）”；10-17 五条建议“全照建议”，看过样稿：“照样稿做吧。做的精致一些。”

原稿在 claude.ai 私有画布「P5-124 远程开 agent 样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里；画布上的 `Main` 即 `A-Menu`）。配色取 Lagoon 近似值，实现时从主题取色；机器名、目录是编的，D 里的错误文字是占位。

| 文件 | 内容 |
| --- | --- |
| `A-Menu.dc.html` | 远程视图的底部菜单：`New Agent…` 能点（⇧⌘N），New Shell、New Workspace 照旧灰、右边 `This Mac` |
| `B-CapsuleMenu.dc.html` | 机器胶囊右键：最上面加 `New Agent on 5090…`，分隔线下是 P5-123 的四行 |
| `C-Dialog.dc.html` | 精简对话框：标题 `New Agent on ● 5090`；Agent 一排胶囊（只列找得到的）、Directory 输入框和最近目录、Name 留空时占位字写会用的名字；Cancel、Start ⌘↩ |
| `D-States.dc.html` | 三种情况：查远端时 `Checking…`、Start 灰；起不来时错误原文显示在框里、框不关、目录框标红；一种都没找到时写明、Start 灰 |
| `E-Offline.dc.html` | 机器没连上时底部菜单的 `New Agent…` 灰，右边写短状态（如 `Connecting…`） |

## P5-124c 自己配命令

用户 10-17：“我觉得是不是可以诶用户提供一个自己配置远程agent的方式（remote new agent那个对哈框中）”；选了“Command 行 + Custom…胶囊”，看过样稿：“可以。按照设计做吧，但是要精致”。

| 文件 | 内容 |
| --- | --- |
| `F-Command.dc.html` | Agent 和 Directory 之间多一行 `Command`：选探测到的那种时预填它的命令、可改；改过的写 `Edited, kept for 5090 · Reset to pi`；自定义的胶囊（虚线图标）悬停出 `×` |
| `G-Custom.dc.html` | 选 `+ Custom…`：Command 自己填（占位 `amp, or ~/bin/my-agent --flag`），起成功后变成一颗胶囊记在那台机器上 |
