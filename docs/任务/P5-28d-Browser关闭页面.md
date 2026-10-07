# 任务：Browser 关闭页面，回到空白页

2026-10-07，paddock/main。
类型：功能变更
依据：用户 10-07：“现在我觉得browser缺少一个功能，就是现在我导航到那个网址之后，我想关闭这个，就像chrome的空页面那种。能不能实现这个功能？”主控给了做法和两个选项，用户选：按钮放在工具栏最右边，用 ×；不配快捷键，只用按钮。
依赖：P5-28c

## 先读
- `AGENTS.md`「规矩」一节，以及「开发方式」里的看板约定。
- `docs/DESIGN.md` §13「P5-28 右侧栏 Browser（WKWebView）」：第 3 条和「P5-28d 关闭页面」。
- 代码：`app/src/browser_view.rs`（`opened`、`pending`、`visited`、`toolbar()`、`content()`），`app/src/browser.rs`（`Page`、`impl Drop for Page`、`Keys`），`app/src/window.rs`（订阅 `Visited`、`Handoff`，存 `right.url`）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-28d-browser-close`，分支 `p5-28d-browser-close`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-28d-browser-close`。
- 可以改：`browser_view.rs`、`browser.rs`、`window.rs`（只接线）。

## 要做的
1. 工具栏最右边，“在默认浏览器打开”按钮的右边，加一个 × 按钮，悬停说明 “Close page”。只有打开过网页时能点；空白页时变灰，和别的按钮一样。
2. 点了以后回到空白页（“Open a page”），地址栏显示 “Enter an address”。具体是：
   - 网页停掉：把整个网页丢掉，下次打开地址时再新建一个。系统没有只清前进后退记录的接口，所以这样前进后退记录也一起清空了。
   - 查找栏、下载提示、失败状态一并清掉。
   - 布局里记的网址清掉，重启后也是空白页。
   - 网页有键盘时点 ×，键盘交给当前的终端分屏，照右侧栏收起时的做法。
3. 登录、cookie 等网站数据保留，不碰网站数据。
4. 界面文字英文；只从主题取色；字号用 `ui.px`。

## 怎么算做完
- 用户原话：“我导航到那个网址之后，我想关闭这个，就像chrome的空页面那种。”
- 验证：
  - 测试：关闭后布局记的网址变成没有；关闭后 `Keys` 不再记在网页上。先写，确认改之前失败。
  - 在 `app/` 下跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 点按钮这一步不能模拟，留给用户实际操作。

## 不要做
- 不配快捷键，⌘W 照旧关终端分屏（用户定）。
- 不做“清除网站数据”；不改工具栏已有按钮的样子和顺序。
- 截图、模拟按键鼠标、测试窗口照 AGENTS.md「桌面窗口测试」。
