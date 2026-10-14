# 任务：New Agent 底边把 agent 种类和模型·强度分成两块

2026-10-14，paddock/main 记。
依据：用户 10-14 试 P5-98 后：“现在pi omp amp等放到这个地方是不是会让人有误解，我是一直觉得那个地方是选模型和effort的地方。”主控给了甲（拆成两块）、乙（一块里加种类名）、丙（不改），用户：“选甲，出样稿吧”；看过样稿、问清顶上预设那排是什么之后：“明白了，上面那个暂时不用改，照你的设计作”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-99-kind-model`，分支 `p5-99-kind-model`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-99-kind-model`。
- 动 `app/src/new_agent_view.rs`、`docs/DESIGN.md`（P5-43 那条注明被改，§13 加 P5-99）和本文件。

## 样稿
`docs/设计稿/P5-99-种类和模型分开/`（原稿在 claude.ai 私有画布「New Agent种类与模型拆开样稿」）：1 选 Claude、2 种类小块点开、3 模型小块点开、4 选 amp。

## 要做的
1. 底边原来的一块（种类图标 + `模型 · 强度`，点开同时选三样）拆成两块，挨着放：
   - **种类小块**：种类图标 + 种类名（`Claude`、`Codex`、`pi`、`omp`、`amp`）+ 下箭头。点开只列五家，和现在选单上半截一样（图标、名字、选中打勾，pi、omp、amp 后面淡字 `own defaults`）。
   - **模型小块**：`模型 · 强度` + 下箭头（字体和现在小块上的一样）。点开只有 Model、Effort 两排，和现在选单下半截一样。
   - 选的种类没有模型可选（pi、omp、amp）时，模型小块不显示。
2. 切换种类的规则照旧（现在 `pick_tool` 的做法：换成那一家的默认模型和强度）；预设、`edited`、Update preset、手改命令同步都照旧。
3. 两个弹出框各自挂在自己的小块下面，开着的那块描边变亮，和别的小块一样。
4. 顶上预设那排不改（用户：“上面那个暂时不用改”）。

## 怎么算做完
用户原话：“照你的设计作”，即照样稿 1～4 做出来、装上给用户看。

## 不做
- 预设那排的名字、默认收起等（用户：“上面那个暂时不用改”）。

## 完成记录（10-14，主控）

### 做了什么
- `new_agent_view.rs`：`Menu` 多一个 `Model`。底边原来的 agent 小块只写“图标 种类名 ▾”；旁边新加 `chip-model` 小块写“模型 · 强度 ▾”，种类没有模型（pi、omp、amp）时不画。原来的 `picker_rows` 拆成 `kind_rows`（五家，点了一家就关上选单）和 `model_rows`（Model、Effort 两排，开着可连着选，去掉了原来分隔上下半截的横线）。种类选单宽 280（和项目、位置选单一样），模型选单照旧 318。切换种类仍走 `pick_tool`，预设、`edited` 不变。
- `docs/DESIGN.md`：P5-43 那条注明 P5-99 起拆开，末尾加 P5-99。

### 验证了什么
- 纯界面改动，没有新逻辑，没写先失败的测试（按 CLAUDE.md 轻量 TDD 第 5 条）。`cargo test --all-targets`（库内 505 条和各集成测试全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。
- 没起窗口、没截图，样子留给用户装上看。

### 拿主意的地方
- 种类选单点了就关：只选一样，和项目、位置选单一致；模型选单要选两样，照旧开着。
- 模型选单开着时种类换成没有模型的那几家，碰不到（选单开着时底下有一层挡着，预设和命令框都点不到），没加保护。

### 没做的
- 顶上预设那排（用户：“上面那个暂时不用改”）。
