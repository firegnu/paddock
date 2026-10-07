# 任务：Browser 关闭页面，回到空白页

2026-10-07，paddock/main。
类型：功能变更
依据：用户 10-07：“现在我觉得browser缺少一个功能，就是现在我导航到那个网址之后，我想关闭这个，就像chrome的空页面那种。能不能实现这个功能？”主控给了做法和两个选项，用户选：按钮放在工具栏最右边，用 ×；不配快捷键，只用按钮。
依赖：P5-28c
待用户：在真窗口里点一下工具栏最右的 ×，看会不会和上方收起右侧栏的 × 混。

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

## 完成记录

- **做了什么**：工具栏最右加 ×（`Icon::Close`，悬停 “Close page”），只在打开过网页时能点。点了以后 `BrowserView::close()` 丢掉网页、马上新建一个空白的隐藏网页（轮询循环沿用原来的，不另起），清掉状态、失败、查找栏、下载提示，回到 “Open a page”。`Visited` 改成 `Option<String>`，关闭时发 `Visited(None)`，`window.rs` 据此把布局里的 `right.url` 清掉。`Keys::close()` 把键盘记录交还 paddock，Browser 原来有键盘时发 `Handoff::ToPane`，由窗口交给当前分屏（和右侧栏收起时同一条路）。网站数据不碰。
- **验证了什么**：新测试 `closing_the_page_gives_its_keyboard_to_the_pane`（从网页、地址栏、查找栏关闭，键盘都回到分屏；终端有键盘时不动）。先用返回 `false` 的空实现跑，失败在“键盘仍记在网页上”，实现后通过。`app/` 下 `cargo test --all-targets`（库 234 项）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。用临时 HOME、`PADDOCK_NO_ACTIVATE=1` 起 debug 版，预写布局开在 example.com，截了自己的窗口：× 在“在默认浏览器打开”右边，样子和别的按钮一致。截图在 scratchpad，不入库。
- **拿主意的地方**：关闭时立刻新建空白网页，而不是等下次输入地址时再建，这样轮询循环不会多起一个；“布局记的网址清掉”没有单独写单元测试：这是视图到窗口的接线（`Visited(None)` → `right.url = None`），仓库里没有 GPUI 视图测试的设施，靠类型改动保证，`right.url` 为空的存取已有测试。
- **没做的事**：点 × 这一步没在真窗口里点（不能模拟点击），留给用户。注意：这个 × 正好在右侧栏右上角那个 ×（收起右侧栏）的正下方，两个 × 上下挨着，用户实际看看会不会混。
