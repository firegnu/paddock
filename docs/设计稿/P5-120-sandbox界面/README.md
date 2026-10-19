# P5-120 sandbox 界面样稿

用户 10-19 看过后：“其他我都同意”“我的理解是不要动ranch的代码。其他都同意”。即：四处都做（Settings 只看）、被拦记录用 C1 卡片浮窗、预设记住 sandbox 选择、ranch 不动。

原稿在 claude.ai 私有画布「P5-120 sandbox 界面样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。配色取 Dune 的近似值，实现时一律从主题取色；agent 名、被拦记录是编的，Settings 里的规则取自本机 10-19 `fence check --profile dev --kind claude` 的输出。

| 文件 | 内容 |
| --- | --- |
| `Main.dc.html` | A New Agent：底边皇冠左边一把锁，选了 `dev` 变亮写档案名；点开菜单 None／`dev`（说明照 fence 原文）、底下 “Checked for Claude in paddock · See the rules”；Will run 里多 `--sandbox dev` |
| `States.dc.html` | A2：1 默认（锁淡、悬停 No sandbox）；2 Controller 开着锁变灰（悬停说明原因）；3 Codex 选 `dev` 时 fence 的警告照原文显示在底边下、锁带黄点；4 种类不支持时退回 None 锁灰，检查失败时锁红、显示 fence 的错误码和原文、Start 灰；5 没装 fence 或远程机器：没有锁 |
| `Card.dc.html` | B：收起的卡片名字和强度后面一把小锁，被拦过时旁边黄色数字（只数 network、file-write）；展开后多一格 Sandbox：`dev · 3 blocked ›`，点了打开 C1 |
| `Denials.dc.html` | C1（**采用**）：从卡片弹出的浮窗，标题 Blocked、agent 名和档案；`Network & writes` / `All` 切换；每行时间、类型签、目标、原文细节；底下一句文件拦截可能不全 |
| `DenialsTab.dc.html` | C2（未采用）：同样的列表放在右侧栏第五个标签 |
| `Settings.dc.html` | D：Settings 新一页 Sandbox，只看：fence 版本状态、档案列表、选种类和目录后显示合并规则（来源层、允许域名、能读能写、不能读写、跳过项、警告） |
| `Delegates.dc.html` | E（P5-141，用户 10-19 同意）：收起的主控卡片名字后面蓝色“箭头指向锁”标记（悬停 `Agents it starts run in sandbox: dev`）；展开详情多一格 Delegates，点开菜单 Not sandboxed／`dev`，底下说明从下一次派活起生效、已在跑的不变、主控自己不进墙；保存中、corral 拒绝、主控重启后的样子写在图下 |
| `Lock.dc.html` | F（P5-145，用户 10-19：“选B，展开卡片和New Agent的锁都统一”）：现在的灰锁对照 A 大锁、**B 小胶囊（采用）**淡青绿底，锁、档案名、黄色 `· 2+`，没拦过只有锁和档案名、C 压在图标角上；最下面一排 B 和主控的蓝色 Delegates 标记并排，看两者分不分得开 |
