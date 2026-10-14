# 任务：New Agent 模型选单改成一列、强度改成带动画的滑块，Settings 的 Models 一节精修

2026-10-14，paddock/main 记。
依据：用户 10-14 看了 P5-100 装好的样子，发来 Settings › Agents › Models 一节和 New Agent 模型选单（Codex、Claude）三张截图：“这几个页面不能做的更加好看精致一些吗？”主控出样稿 `docs/设计稿/P5-101-模型选单精修/`（A 一列、B 两列、C Settings、D 滑块），用户：“effort改成滑块，滑动的时候随着effort的大小最好有点动画。要兼容claude和codex的effort调整。codex默认astra high. claude code默认opus5.5。 settings就按照你的设计出”；问过后选“A 一列”、Claude 默认“opus[1m]（照旧）”；看过 D：“可以，写任务文件开始做吧”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-101-picker-look`，分支 `p5-101-picker-look`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-101-picker-look`。
- 动 `app/src/new_agent_view.rs`（模型选单）、`app/src/settings_models.rs`（Models 一节）、可能 `app/src/models.rs`（档位、颜色这类纯计算），测试、`docs/DESIGN.md`、样稿目录和本文件。

## 要做的

### 一、模型选单（样稿 A、D）
- 模型一个一行，名字靠左（等宽字），选中的那行淡强调色底、右边一个勾；默认的那个模型名字后面写小字 `default`；悬停的行淡淡亮一下。
- 选单顶上一行：左边 `Model`，右边小字写列表从哪来（`from codex · Oct 14`／`config.toml`／`paddock's list`）。
- 模型和强度之间一条细线。

### 二、强度滑块（样稿 D）
- 强度从一排小格改成滑块：一条轨道，每档一个小点，下面写各档名字，右上角写当前档。拖动或点轨道、点下面的名字都能选；只停在档位上。
- 档位跟着选中的模型：Claude low～max 五档；Codex 按模型，有 `ultra` 的六档、没有的五档。换模型时原档位新模型也有就留，没有用 `high`，再没有用最后一个（P5-100 的规则不变）。
- 动画随强度大小：
  - 拇指换档时滑过去，带一点回弹；
  - 轨道已走过的部分和拇指的颜色随档位从强调色（薄荷绿）变暖到黄、再到偏红（同一档名在 Claude 和 Codex 颜色一样，比如 `max` 都一样）；右上角的档名和底边模型小块里的强度字跟着变色；
  - 档位越高拇指越大、光晕越强；从 `high` 起拇指外圈一圈圈向外扩散，档位越高扩散越快；`low`、`medium` 不扩散。
- 颜色从主题里取（强调色、黄、红），换主题跟着变。

### 三、默认
- Codex 默认 `gpt-6-astra`、`high`（P5-100 已做，不变）。
- Claude 默认 `opus[1m]`、`high`（照旧）。

### 四、Settings › Agents › Models（样稿 C）
- 去掉表头那行（Agent／In New Agent／From）。
- 每行左边是名字，名字下面小字写来源；来自 codex 的前面一个绿点，问 codex 失败时点是黄的、下面黄字 `Couldn't ask: <原因>`；正在问时写 `Asking codex…`。
- 中间模型改成可换行的小标签，全部显示、不截断；默认的那个用强调色并写 `default`。
- Codex 一行右边 `Refresh` 带刷新图标，问着时变灰不能点。
- 行间分隔线和文字对齐。
- 下面说明缩成一句：`Codex is asked once a month. To use your own lists, add [agent_models.claude] or [agent_models.codex] to config.toml.`

## 怎么算做完
用户原话：“这几个页面不能做的更加好看精致一些吗？”“effort改成滑块，滑动的时候随着effort的大小最好有点动画。要兼容claude和codex的effort调整。codex默认astra high. claude code默认opus5.5。 settings就按照你的设计出”（Claude 默认问过后定“opus[1m]（照旧）”，模型列表选“A 一列”）。

## 完成记录（10-14，主控）

### 做了什么
- `new_agent_view.rs`：模型选单改成一列（选中打勾、默认标 `default`、顶上写来源）；强度改成滑块：轨道、档点、档名，按／拖／点档名选档；换档动画（滑过去、冲过一点再回来，280ms）、颜色随档变暖（主题强调色→黄→红，按档名，两家一致）、档越高拇指越大光晕越强、`high` 起外圈扩散且越高越快；底边模型小块里的强度字也跟着变色。打开选单时不从上次位置滑进来。
- `new_agent.rs`：表单多两个只读的：默认模型、列表来源。
- `models.rs`：来源写成 `from codex, Oct 14`（选单和 Settings 同一句）；去掉不再用的 `List::shown`。
- `settings_models.rs`：照样稿 C 重画。

### 验证
- 先写测试看红再实现：档名决定冷热（`max` 在 Claude、Codex 一样热；不认识的名字按位置）、鼠标位置找最近的档（越界取两头、宽度为 0 或没有档时取 0）、颜色从强调色经黄到红、回弹曲线冲过不到 12% 并停在终点；表单的默认模型和来源；来源文字。
- `cargo test --all-targets`（库内 521 条及各集成测试）全过；`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都干净。
- 没起窗口看：滑块动画、拖动手感、Settings 的样子要用户实际看（规矩不让脚本模拟鼠标，选单要点开才看得到）。

### 拿主意的地方
- 颜色用主题的强调色、黄、红三色插值，没写死样稿里的颜色，换主题跟着变。
- 外圈扩散从 `high` 起（样稿一样），一圈 2.6 秒到最快 0.7 秒；30fps 封顶，和侧栏的脉冲一样。
- 手写命令里写了列表里没有的强度时，滑块不画拇指，只画档点和档名。

### 没做
- 选单里模型的说明文字（codex 目录里有，没存）。
