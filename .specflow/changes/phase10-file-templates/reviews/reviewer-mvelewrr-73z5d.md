# QA Report: reviewer-mvelewrr-73z5d

## Verdict

`repair`（仅 1 项 regression：index.html 远程 Figtree；上轮 missing_test 已修复）

## Spec Coverage

- 上轮退回项（COMMIT 失败注入测试）：`storage/backup.rs:305` 在 `between()`（文件已移入）之后、`commit()` 之前通过 `#[cfg(test)]` 的 `fail_import_commit` 注入失败；`fail_import_commit` 字段生产路径恒为 false。测试 `commit_failure_after_files_are_moved_restores_database_and_template_dir`（service/backup.rs:1497）覆盖“覆盖已有文件 + 新建目录 + 新文件”，断言 DB dump、模板目录 tree（含哈希）、旧文件原位、新目录移除、import-staging 无遗留，且失败后可重新校验并导入成功。Result: pass（四条不变量均成立）。
- 其余需求（设置视图 S1、文件与导出、exportDir 校验、关于并入、zip 导出/导入校验、AC-36/37、重置、日志隐私）：沿用上轮结论，本轮实现仅增加测试注入点，未改逻辑；cargo test 全绿。Result: pass。

## Findings

1. Severity: low
   - Category: `missing_test`（已解决）
   - 上轮 COMMIT 失败注入测试缺失，本轮已补，不再成立。

2. Severity: medium
   - Category: `regression`
   - Expected: SPEC-20 起应用不加载任何远程字体，`index.html` 无 Figtree / fonts.googleapis.com / fonts.gstatic.com 引用，description 不含“MacOS 风格”（visual-qa-checklist“Figtree 零引用”；本地优先应用不应在启动时访问外网）。
   - Actual: `index.html:8` description 含“MacOS 风格”；`index.html:10-12` 两条 preconnect（fonts.googleapis.com、fonts.gstatic.com）与 Figtree stylesheet 仍在。`git status` 显示 index.html 未修改。E2E 无页面不引用 googleapis/gstatic 的断言（`grep -rn "googleapis|gstatic" e2e src` 无结果）。
   - Reproduction / evidence: `sed -n 8,12p index.html`。
   - Required repair（人工决策 2026-10-19，contracts/24 允许修改范围已含 index.html，仅限此项）：删除两条 preconnect 与 Figtree stylesheet 及其注释；description 去掉“MacOS 风格”；在 E2E 增加断言“页面不引用 fonts.googleapis.com / fonts.gstatic.com”（如读取 document.documentElement.outerHTML 并断言不含）；重新跑 npm run build 与 .specflow/bin/e2e-isolated.sh。

## Tests Reviewed

- `cd src-tauri && cargo test` — 218 passed, 0 failed, 1 ignored（本轮复跑，含新增 commit 失败测试）。
- `npm run build` — 通过（仅 chunk 体积警告）。
- `.specflow/bin/e2e-isolated.sh` — 本轮未重跑（实现者本轮未改前端/E2E；上轮 6 passing）。
- 500 MB 性能：引用 implementer 记录（release 导出 9.27 s、导入校验解压 0.49 s），未重跑。

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
- [ ] 未通过：字体经 `@font-face` 本地加载；**`index.html:10-12` 仍引用 Figtree（Google Fonts）**，违反“Figtree 零引用”；见 Finding 2（regression，人工决策 2026-10-19 纳入 SPEC-24 修复）。
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


## Next Step

- `repair` → 仅将 Finding 2（regression，index.html 远程字体）退回 Implementer；修复后复审。
