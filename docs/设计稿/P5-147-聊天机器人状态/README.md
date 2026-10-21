# P5-147 Settings 聊天机器人状态样稿

用户 10-21 看过后：“选甲，开始做吧”。

原稿在 claude.ai 私有画布「P5-147 Settings 聊天机器人状态样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。配色取 Dune 的近似值，实现时一律从主题取色；工作区名、频道 ID、时间都是编的，字段取值照 ranch `docs/CONTRACT.md`「`pony status`（R38）」。

| 文件 | 内容 |
| --- | --- |
| `Main.dc.html` | 甲（**采用**）：Settings 新一页 `Chat`，排在 Dispatch 后面。顶上 pony 版本和运行状态、开始时间、pid、后台服务装没装，右边一句只看；一句说明 pony 是什么；每家聊天软件一张卡：名字、连接状态和持续多久、工作区和机器人名；`Channels · N` 逐行 `#频道`（私有标锁）`→ 仓库`、`@pony goes to <仓库>/main`，没查到名字时显示频道 ID，有问题的黄字说明 |
| `Alt.dc.html` | 乙（未采用）：放在 Agents 页最底下一节，每家一行，频道问题只给个数 |
| `States.dc.html` | 甲的各种状态：1 没装 pony；2 `config: missing`；3 没在跑（`last_exit.message` 原文、后台服务没装时怎么装）；4 `config: invalid`（`config_message` 原文，在跑的继续用旧配置）；5 连接中（频道只列 ID）；6 连不上（原因代码加 `message` 原文，不认识的代码同样显示）；7 `other_connections > 0` 的黄色警告；8 不认识的 `kind`、`connection`、`problem` 照原名显示 |
