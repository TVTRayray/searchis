# QA Report: reviewer-mvel6cvj-qxi7n

## Verdict

`repair`（仅 1 项 missing_test，实现本身未发现缺陷）

## Spec Coverage

- 设置视图 S1 / 文件与导出 / 关于并入：E2E 用例“navigates the settings view…(SPEC-24)”通过；`AboutModal`/`SettingsModal` 在 src 中零引用（仅 e2e 断言其不存在）。Result: pass。
- exportDir 校验（VALIDATION_FAILED）、明文说明、模板目录只读路径与打开：单元测试 + E2E 覆盖。Result: pass。
- zip 导出/导入校验（`../`、绝对路径、反斜杠、符号链接、多余/缺失、哈希、上限）、v1 JSON、冲突、分组合并、幂等 token、启动清理：cargo test 217 passed。Result: pass。
- AC-36 往返哈希一致、AC-37 恶意包：单元 + E2E 通过。Result: pass。
- 重置：确认文案（SettingsView.tsx:660）写明模板文件会被删除；实现先把模板目录改名为 `templates-reset-<uuid>`，`reset_all` 单事务成功后删除，失败改回（`reset_examples`，service/backup.rs:520）；有失败恢复与启动恢复测试。`reset_examples` 清空全部条目与分组符合 FR-DAT-03 与 contracts/24。Result: pass。
- 日志隐私：备份模块无 log/tracing 调用（唯一 println 在 `#[cfg(test)]` 性能测试内）。Result: pass。

## 偏差裁定：文件移动发生在 COMMIT 之前

按 Planner 的四条不变量核对：
1. COMMIT 失败时撤回已移入文件：代码成立。`apply_import` 中 `transaction.commit()` 返回 Err → `run_import` 的 Err 分支调用 `ops.undo()`（删除新放入文件、旧文件挪回、移除本次创建的空目录），暂存目录由 `import_commit_with` 清理。**但无测试覆盖**。
2. 移动中途失败 DB 回滚、已移动文件撤回：成立，测试 `injected_failures_at_every_stage…`（stash/place/after_place）断言 DB dump、模板目录树（含哈希）、leftovers 均与导入前一致。
3. 补偿路径不删除导入前已存在的用户文件：成立。`fsops::move_file` 在目标已存在时直接报错，故 `placed` 只记录本次成功放入的文件；被覆盖的旧文件先经 stash 挪走并在 undo 中挪回；仅删除本次创建的空目录（`remove_dir`）。
4. 注入测试覆盖：移动中途失败已覆盖；**COMMIT 失败的注入缺失**——`check_injected_failure()` 位于 `between()` 之前，`after_place` 在 `between` 内、COMMIT 之前，二者都不会触发 `commit()` 的 Err 路径。

结论：条件 1 的实现成立但条件 4 未满足，按 Planner 标准退回补测试。

## Findings

- Severity: medium
  - Category: `missing_test`
  - Expected: 有注入 COMMIT 失败的测试，断言 DB、模板目录（逐字节/哈希）与导入前一致，且 import-staging 被清理。
  - Actual: 仅有 stash/place/after_place（均在 COMMIT 之前）与 DB 写入失败的注入；`transaction.commit()` 失败路径无测试。
  - Reproduction / evidence: `storage/backup.rs` apply_import 中 `check_injected_failure()` → `between()` → `commit()`；`service/backup.rs` 测试 `injected_failures_at_every_stage…` 的 stage 列表。
  - Required repair: 在 `apply_import` 的 `commit()` 处增加测试注入点（如 `#[cfg(test)]` 在 between 之后、commit 之前使 commit 路径返回 Err，或新增 stage `commit`），并在现有回滚测试中加入该阶段（需覆盖“覆盖已有文件 + 新建目录 + 新文件”场景，断言 `tree`、`dump`、`leftovers` 不变，且之后可重新导入成功）。无需改动实现逻辑。

## Tests Reviewed

- `cd src-tauri && cargo test` — 217 passed, 0 failed, 1 ignored（本轮复跑）。
- `npm run build` — 通过（仅 chunk 体积警告）。
- `.specflow/bin/e2e-isolated.sh` — 6 passing，含 SPEC-24 用例（隔离显示运行，未碰真实桌面）。
- 500 MB 性能：引用 implementer 记录（ignored 测试，未重跑）：release 导出 9.27 s、导入 0.49 s；界面异步 + 进度遮罩。

## Design Conformance

按 docs/design/visual-qa-checklist.md 逐条（证据为本轮静态检查 + 上文 E2E/构建结果）：

### 一致性
- [x] 单一 token 体系：`grep -rnE "#[0-9a-fA-F]{3,8}\b|rgba?\(" src/components src/App.tsx` 无命中。
- [x] 强调色/圆角/字体一致：SettingsView 复用 manager-* 类与 token，`index.css` 本次新增行仅用 `var(--radius-pill)`、`inherit`。

### 主题与对比度
- [ ] pending_human：明/暗双模设置视图截图（tasks 5.9）。
- [ ] pending_human：对比度 ≥4.5:1 需实机/截图量测；新增样式只引用既有 token，未引入新色值。
- [ ] pending_human：主题切换无白闪、重启首帧；本变更未改 ThemeProvider（localStorage 仅作首帧缓存，真相源在后端）。

### 字体与圆角
- [ ] 部分通过：字体经 `@font-face` 本地加载；**`index.html:12` 仍引用 Figtree（Google Fonts）**，违反“Figtree 零引用”。该行为既有遗留（最后改动 a26c81a，非本 change），记为 Finding（low，out-of-scope，不阻断）。
- [x] 圆角：`border-radius` 除 `inherit`、`var(--radius-*)` 外仅 `.quick-search-window mark` 的 3px（高亮标记，既有，非本 change）；本次新增无规则外圆角。
- [x] 中文无斜体：italic 仅出现在 Playfair `@font-face` 声明，无 `font-style: italic` 使用；衬线仅用于标题（本次未新增）。

### 动效与无障碍
- [x] 动效有动机：进度条宽度过渡用 `--motion-manager`(220ms ≤300ms)，不确定进度动画为状态反馈；无其他新增动效。
- [x] reduced-motion：新增 `.settings-progress[data-indeterminate]` 在 reduced-motion 下 `animation:none`，全局规则 `index.css:326` 令其余瞬时。
- [x] 焦点可见：全局 `:focus-visible`（index.css:191）；设置控件复用 `manager-control/button`；E2E 键盘路径通过（部分为 Tab 顺序，未做完整键盘逐项审计）。

### 状态完整性
- [x] loading/error：导入/导出 withBusy 进度遮罩（`backup-progress`）；错误对话框带标题与错误码文本，exportDir 校验 `role="alert"`；无效包列出错误清单。empty：设置视图无列表，不适用。
- [x] 导出路径提示可见：`last-export`、默认导出目录/自定义提示、模板目录路径与“明文存放”说明（E2E 断言）。文件缺失/自动导入标记不在本 spec 范围。
- [x] 信息密度：设置视图分区卡片，无过度堆叠（E2E 通过）；精细视觉密度 pending_human。

### 隐私与安全（视觉侧）
- [x] 敏感遮挡/编辑器不挂载：本变更未触及该逻辑，E2E 既有用例通过。
- [x] 文件模板无敏感开关、不预览内容：设置视图仅显示目录路径，不读文件内容。
- [x] 日志/提示：备份模块无 log/tracing 调用；错误提示仅含错误码文本，导出成功提示中的路径为允许例外。

### 回归
- [x] 键盘语义：E2E 6 passing 覆盖创建/搜索/设置；设置输入 Enter 应用 exportDir（SettingsView.tsx:424）。IME 组合态未改动。
- [x] 设置持久化走后端契约：`onUpdateConfig('exportDir')` → 后端校验；`localStorage` 仅 ThemeProvider/main.tsx 的首帧主题缓存，SettingsView 零使用。

### Searchis 专项
- [x] `grep -rn "backdrop-filter" src/` 无结果。
- [x] `index.css` token：本次 diff 55+/18−，仅新增 settings-* 样式，未改 token 定义；与 design-tokens.md 逐项比对 pending_human（未逐值核对）。
- [x] 无未引用组件：`src/components` 下 CodeEditor、FileTemplateDetail、FileTypeIcon、GroupDialog、GroupTree、NewTemplateDialog、PlusMenu、SelectField、KeyboardShortcutsModal、ManagerWindow、QuickSearchWindow、SettingsView、ThemeProvider、command 均有 import；`HeaderBar|AboutModal|SettingsModal` 在 src 零引用。
- [x] 纸张颗粒层 `pointer-events:none`（index.css:230）。
- [ ] pending_human：快搜结果行 Q-A 与徽标规则——本 change 未触及快搜窗口，需实机确认无回归。
- [x] 每界面最多一个主按钮：导入预览对话框仅 `import-confirm` 为 primary，覆盖为 danger；设置页其余为 secondary。
- [ ] pending_human：KWin 26px 圆角、KDE Global Menu 五组菜单（实机）。

### 新增 Finding（不改变 verdict）
- Severity: low · Category: `design_conformance`（既有遗留）· `index.html:12` 引用 Figtree 字体，违反 checklist“Figtree 零引用”；非本 change 引入，建议另开清理，不阻断 SPEC-24。

## Next Step

- `repair` → 仅将上述 missing_test 退回 Implementer；修复后复审。
