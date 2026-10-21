# 任务：Settings 里显示 Slack 机器人的状态（从聊天软件 @ agent 开发，主体在 ranch）

2026-10-21，paddock/main 交给 paddock/dev-chat（Claude Code，opus[1m] / high，套 fence `dev` 墙）。
依据：用户 10-19：“再加一个draft task，可以接入slack等社交软件，通过社交软件AT的方式进行交互开发”。10-20、10-21 和主控谈定了做法和范围（见文末「来历」）；10-21 定机器人主体移到 ranch（R38），本件只做 paddock 的 Settings 那块。ranch R38b 已把 `pony status` 写进 ranch CONTRACT（本文件附录照抄），用户 10-21 看了样稿：“选甲，开始做吧”。本轮只做只读显示，不改 ranch、不写 pony 的配置。
执行：派出
（理由：新加 Settings 一页和读 pony 的模块，照现有 Sandbox、Dispatch 页的路子做，样子用户已选；按分工表中档「界面和设计」读 `middle-design` 格。主控审，装上给用户看。）
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.98）、拿不准、拿不准；交叉审查和影响面主控定：只读显示，pony 自己不输出凭证和消息内容，做错了只影响这一页）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
优先：低
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- 本文件末尾附录：ranch CONTRACT「`pony status`（R38）」一节原文，命令、找法、字段、取值、怎么显示都以它为准。墙里读不到 ranch 仓库。
- `docs/设计稿/P5-147-聊天机器人状态/README.md` 和 `Main.dc.html`（甲，采用）、`States.dc.html`（各种状态）。`Alt.dc.html` 是没选的乙，不做。
- `app/src/fence.rs`、`app/src/settings_sandbox.rs`（P5-120、P5-160）：找外部命令、看 `--version` 的 contract、后台调命令、解析 JSON、没装时怎么显示、测试怎么用假脚本。照这个路子做。
- `app/src/settings.rs` 的 `Page`；`app/src/settings_view.rs` 怎么挑页面、窗口重新变活跃时刷新；`app/src/settings_dispatch.rs` 可以参考新页怎么接进来。
- `docs/DESIGN.md` §13 讲 Settings › Sandbox（搜 `P5-120`）和 Dispatch（搜 `P5-161`）的那几条。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-147-chat`，分支 `p5-147-chat`（已从 main 建好）。主要动 `app/src/settings.rs`、`app/src/settings_view.rs`，新建读 pony 的模块和这一页的模块（名字开发定），测试放模块里或 `app/tests/`；`docs/DESIGN.md` §13 补一段。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-147-chat`（从 `.target/main` 克隆的）。

## 要做的
照样稿甲做 Settings 新的一页 `Chat`，排在 Dispatch 后面、Diagnostics 前面，只看不改：
1. 找 pony、确认版本：照附录（PATH 上的 `pony`，再 `~/.local/bin/pony`；`pony --version` 的 `contract` 是 `"1"` 才按附录解析）。两处都没有，或 contract 不对：整页一句话说没装或版本不对、pony 随 ranch 来（样稿状态 1）。
2. 顶上一行：pony 版本、在不在跑（在跑时开始时间、pid）、后台服务装没装；右边一句只看、配置在 `~/.pony` 改。下面一句说明 pony 是什么、只有绑了频道的仓库会发到聊天里。
3. 各状态照附录「paddock 怎么显示」和样稿 `States.dc.html`：
   - `config: missing`：这台机器没配置，pony 只在配置了的那一台跑（状态 2）。
   - 没在跑：`last_exit.message` 原文；`service: not_installed` 时说怎么装（`pony service install --dry-run`，再去掉 `--dry-run`），`unsupported` 照实说（状态 3）。
   - `config: invalid`：`config_message` 原文；在跑的说明还在用旧配置（状态 4）。
4. `chats` 每项一张卡，标题用 `kind`（`slack` 显示 `Slack`，不认识的照原名）：
   - 连接状态：`connected`、`connecting`、`failed`、`off`，加上持续多久（`since`）；`failed` 时原因代码和 `message` 原文，不认识的代码照样显示（状态 5、6）。不认识的 `connection` 照原名、中性颜色显示（状态 8）。
   - 连上过的写工作区和机器人名（`workspace`、`bot`）。
   - `other_connections > 0`：黄色警告，可能另一台机器也开着 pony、消息可能被它收走（状态 7）。
   - `Channels · N`：每项一行 `#name`（`name` 是 `null` 就显示 `id`），`private` 为真标锁，`→ repo`，旁边 `@pony goes to <repo>/main`；`problem` 不是 `null` 的黄字说明（`not_member`：机器人没在这个频道，要邀请它；`not_found`、`archived` 照实说；不认识的照原名）。
   - `chats` 为空数组（有配置但一家都没配）时说一句。
5. 读的时机：打开这一页时读；这一页开着时定时重读（附录说可以每几秒调一次，间隔开发定）；Settings 窗口重新变活跃时也读。调命令不卡界面（照 fence 的做法放后台）。`pony status` 退出码不是 0 或输出不是 JSON：显示它的报错，不崩。
6. 照附录：没写的字段忽略，不认识的取值照样显示；照 `chats` 列表画，不写死 `slack`。

## 怎么算做完
- 用户 10-21 定的验收（第 7 条）：paddock 的 Settings 里能看到它连没连上、绑了哪些频道。
- 验证只做这些：用假 `pony` 脚本（照 `fence.rs` 测试的写法）写针对测试，覆盖 contract 不对、`config: missing`、没在跑、`failed`、频道有 `problem`、不认识的取值这几种读出来的结果；`cargo test --all-targets --no-fail-fast`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各一次，加 `git diff --check`。墙里 `view::links::tests::a_shell_s_relative_paths_are_from_where_it_has_gone_to` 失败是老问题（墙里 `clear` 不清屏），和本件无关，写进完成记录即可。页面长什么样留给用户装上看，你不起 paddock 窗口、不截图。觉得不够，在回复里说，不要自己加。

## 不要做
- 不读、不写 `~/.pony` 里的任何文件；只调 `pony --version` 和 `pony status`，不调 `pony run`、`pony service` 这类会改东西的子命令。
- 不改 ranch，不改 corral 的调用；不加依赖，不加新的主题颜色键，不改用户的主题和设置文件。
- 不做 Settings 里改 pony 配置、绑频道、装服务的按钮（只看）。
- 不把截图放进仓库。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。停自己起的进程用起的时候记下的 PID。
- 不起 paddock 窗口，不截图，不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 碰到要拿主意的事（包括附录没写清、样稿和附录对不上的地方），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的办法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-147-chat` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 来历（不用照做）

### 已定的（用户 10-20、10-21）
- **架构照 ranch 接 agent 的做法，做成插件**（用户 10-20：“这个架构要设计一下，我觉得应该像ranch接不同agent那样的设计，类似插件的形式”）：每家聊天软件一个适配器，登记在一张表里；共用核心不写任何一家的名字；和 agent 只走 corral 公开命令。凭证放本机，不进仓库。
- **放进 ranch**（用户 10-20：“接受你的建议，甲”）：和 corral 一样每台机器一份、后台常驻，不开 paddock 也能用；paddock 只在 Settings 里显示它的状态，读它的 JSON 输出。
- **分两期**（用户 10-20：“同意，分两期”）：第一期 @ 已在跑的 agent 说话、Needs you 和主控新回复在频道提醒、看状态；第二期在聊天里开、停 agent。
- **怎么 @ 到 agent：一个频道对应一个仓库**（用户 10-21 选主控建议的这种，备选“只一个频道、每条写全名”“和机器人私聊”）：频道绑一个仓库，`@机器人 说话` 默认送给 `<仓库>/main`；第一个词是同仓库短名送给它，跨仓库写全名只能送给绑了频道的仓库；线程里回复送给线程开头那个 agent；机器人先回一行 `→ <agent 全名>`。
- **没绑频道的仓库一个字都不发到 Slack**（用户 10-21 选主控建议的这种，备选保留总频道配白名单、照原样全发）：没有总频道；跨仓库指向没绑的回“这个仓库没接入 Slack”；接入按仓库逐个决定，默认不接入；绑了频道的回复原样整段发出去，第一期不做内容过滤。起因：“只有用户能发话”管不到哪些内容会发出去。
- **先接 Slack**（用户 10-21，备选飞书、Discord、Telegram）。
- **第一期只有用户自己能用**（用户 10-21，备选白名单几个人、按人分权限）。
- **第一期只在这台 Mac 上开**（用户 10-21，备选交给 ranch 设计多机）：两台机器连同一个 Slack 机器人，消息会随机落到其中一台。
- **主体移到 ranch**（用户 10-21：“P5-147这个任务整体是不是可以移到ranch中的任务队列中”，选“主体移 ranch，留 Settings”）：ranch 建成 R38（ranch `docs/任务/R38-从Slack等聊天软件at着agent开发.md`）。机器人叫 pony。R38b（ranch main `b4dec26`）写好 `pony status` 约定后开本件（用户 10-21 对 ranch 定）；PATH 上的 pony 要等 R38c 部署，真实数据要等 R38f 上线。
- **样子选甲**（用户 10-21：“选甲，开始做吧”）：样稿 https://claude.ai/artifact/1vQswVKLu3BD7jabPHgYkN ，源文件在 `docs/设计稿/P5-147-聊天机器人状态/`。

### 验收（七条，用户 10-21：“验收照这七条。”）
本件只管第 7 条；1–6 是机器人本身的，跟着需求交给了 ranch R38。
1. 在 `#paddock` 里 @bot 说一句，`paddock/main` 能收到，这一轮的回复贴回同一个线程；在线程里接着回，就是接着和它聊。
2. 某个 agent 变成 Needs you，或者主控有新回复时，对应频道里出现提醒；在提醒的线程里回复，能送到这个 agent。
3. 在聊天里能看哪些 agent 在干活、哪些在等。
4. 没绑频道的仓库，提醒、回复、状态都不出现在 Slack；跨仓库 @ 它会被拒绝。
5. 别人在频道里 @bot，机器人不理。
6. Slack 的凭证只放在本机，不进任何仓库。
7. paddock 的 Settings 里能看到它连没连上、绑了哪些频道。

## 附录：ranch `docs/CONTRACT.md`「`pony status`（R38）」（ranch `b4dec26`，照抄）

`pony status` 是 ranch 的另一条命令（聊天软件机器人 pony，见 `docs/DESIGN.md` §10），不是 corral 的子命令，也不要求 corral 在跑。它只读本机文件（配置、`run/status.json`、锁），不联网、不调 corral、不碰 agent，几毫秒返回，可以每几秒调一次。输出一行 JSON，不输出 token、用户 ID 和消息内容。

找命令：先 PATH 上的 `pony`，找不到试 `~/.local/bin/pony`（launchd、桌面程序常没有 `~/.local/bin` 在 PATH 里），和找 corral 一样。找到后用 `pony --version` 确认，输出 `{"ok":true,"version":"0.1.0","contract":"1"}`；`contract` 不是 `"1"` 的不按这一节解析。两处都没有就是「未安装」。

成功退出 0，`ok:true`。没配置、没在跑、连不上都是 `ok:true` 里的数据，不是错误；只有参数错报 `usage`、退出 2。

```json
{"ok":true,"contract":"1","version":"0.1.0",
 "config":"ok","config_message":null,
 "service":"installed",
 "running":true,"pid":41234,"started":1792550000.5,"last_exit":null,
 "chats":[
  {"kind":"slack","connection":"connected","since":1792550003.1,"reason":null,"message":null,
   "workspace":"firegnu","bot":"pony","other_connections":0,
   "channels":[
    {"id":"C0AAAAAAAAA","name":"paddock","private":true,"repo":"paddock","problem":null},
    {"id":"C0BBBBBBBBB","name":null,"private":null,"repo":"ranch","problem":"not_member"}]}]}
```

| 字段 | 含义 |
| --- | --- |
| `ok` | 成功时为 `true`；参数错时为 `false`，报 `usage`、退出 2。 |
| `contract` | 这一节的格式版本，现在是 `"1"`。 |
| `version` | pony 的版本，只用来显示。 |
| `config` | `ok`、`missing`（没有配置文件：这台机器没开）、`invalid`。 |
| `config_message` | `config` 不是 `ok` 时的说明。`invalid` 时写文件、位置和原因；凭证文件的错不带原文。在跑的 pony 遇到改坏的配置会继续用旧的，这里照样报 `invalid`。`ok` 时为 `null`。 |
| `service` | `installed`、`not_installed`、`unsupported`（非 macOS）。 |
| `running` | 有 `pony run` 拿着锁。 |
| `pid` | 拿着锁的 `pony run` 的进程号；`running` 为真时才有。 |
| `started` | 那次运行开始的时间，Unix 秒（可带小数）；`running` 为真时才有。 |
| `last_exit` | 没在跑时上一次为什么退出：`{"at":…,"code":"secrets_too_open","message":"…"}`；不知道时为 `null`；在跑时为 `null`。`at` 是 Unix 秒。 |
| `chats` | 配置里每家一项，按 pony 的登记顺序。没配置时是空数组。 |
| `chats[].kind` | 适配器，现在只有 `slack`。前端照列表画，不写死。 |
| `connection` | `connected`、`connecting`（第一次连或正在重连）、`failed`（连不上，等着重试）、`off`（pony 没在跑）。 |
| `since` | 进入当前 `connection` 的时间，Unix 秒（可带小数）。 |
| `reason`、`message` | `failed` 时的原因代码和说明；其他情况为 `null`。代码有：`secrets_missing`、`secrets_too_open`、`secrets_invalid`、`auth_failed`、`socket_mode_off`、`network`、`rate_limited`、`slack_error`；以后可能加，不认识的照 `message` 显示。 |
| `workspace`、`bot` | 连上后由 `auth.test` 得到的工作区名、机器人名；没连上过时为 `null`。 |
| `other_connections` | Slack 报的除自己以外的连接数，大于 0 说明可能有另一台机器也开着。 |
| `channels` | 配置里绑的频道，按配置顺序。 |
| `channels[].id` | 配置里写的 Slack 频道 ID，总有。 |
| `channels[].repo` | 这个频道绑的仓库，总有。 |
| `channels[].name` | 频道名；连上并查过之后才有，否则为 `null`。 |
| `channels[].private` | 是否私有；连上并查过之后才有，否则为 `null`。 |
| `channels[].problem` | `null` 或 `not_member`（机器人没被邀请进去）、`not_found`、`archived`；以后可能加。 |

paddock 怎么显示（P5-147 定样子，这里只说数据怎么对应）：

- `config: missing`：「未配置（只在要用的那台机器上配）」。
- 没在跑：显示 `last_exit.message`；`service: not_installed` 时提示怎么装。
- 连没连上：`connection` 加 `reason`、`message`；`other_connections > 0` 时显示警告。
- 绑了哪些频道：`channels` 每项 `#name`（没有就显示 `id`）→ `repo`，有 `problem` 的标出来。
- 遇到没写的字段要忽略，不认识的取值要能显示，和本文开头的规矩一样。

对 paddock：只是兼容新增，不改 corral 的命令、JSON、`~/.corral`，paddock 现有功能不用动。R38b 合并后就可以照这一节开 P5-147（用户 10-21 定）；R38c 部署之前 PATH 上还没有 `pony`，按「未安装」显示。设置页照 `chats` 列表画，不写死 `slack`。
