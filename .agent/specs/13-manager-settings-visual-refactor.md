# SPEC-13 管理窗口与设置视觉重构并入

## 基本信息

- 当前状态：`done`
- 当前责任角色：`Orchestrator`（已放行）
- 关联 PRD：`FR-SNP-01~04`、`FR-MGT-01~05`、`FR-SET-01/02`、`AC-02/06/07/08/11/15/18/19`
- 前置 Spec：`SPEC-12`（done）
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 允许修改：`src/App.tsx` 的主窗口渲染层、`src/components/HeaderBar.tsx`、`ManagerWindow.tsx`、`SettingsModal.tsx`、本 slice 实际使用的 `src/components/ui/**` 与样式
- 禁止修改：`src-tauri/**`、`src/api/snippets.ts` 的 IPC 契约、`src/types/snippet.ts` 领域字段、数据库/设置事务、双窗口路由、`docs/prds/**`

## 目标

把外部重构版的 HeaderBar、ManagerWindow、SettingsModal 外观并入正式主窗口，同时保留真实 CRUD、筛选排序、回收站、敏感遮挡、revision 冲突、系统设置和 XDG/KGlobalAccel 实际状态。

## 成功标准

- [x] 管理页使用新版导航、列表、表单、卡片、toast 和主题样式；Onboarding 未迁移，快捷键帮助仅移除已废止的 Tab/macOS 文案。
- [x] 全部/置顶/最近使用/标签/回收站视图与四种排序的既有本地筛选逻辑保持不变。
- [x] 新建/编辑表单、敏感显隐、复制/粘贴和 revision 请求边界保持原行为；回收站已接通真实 IPC。
- [x] 设置仍由 `settingsApi` 与系统真实状态驱动；未引入 external App 的 localStorage 配置/片段逻辑；快捷键注册失败回滚逻辑保留。
- [x] 快捷键保持 Linux/KDE：Alt+O、Ctrl+Enter、Ctrl+V；目标迁移文件不含 Option/Cmd/⌘。
- [x] 不展示或保存 `playAudioFeedback`；`restoreClipboard` 保持 false 且无可操作开关。
- [x] 成功提示节制、错误明确；新增管理/设置默认文本不含正文。

## 并入规则

1. 外部 `App.tsx` 只作为视觉参考，禁止整文件覆盖；保留当前窗口标签路由、真实 API 状态与错误处理。
2. 外部 mock CRUD/localStorage/初始数据/假系统状态一律不并入。
3. 组件 props 应适配当前 PersistedSnippet/Settings 契约，不反向修改领域类型迎合视觉组件。
4. 本 slice 不处理 OnboardingWizard；KeyboardShortcutsModal 仅按本次 Required Fix 删除已废止的 Tab/macOS 文案，完整清理由 SPEC-14 完成。

## 验收

- [x] `npm run build`、`cargo test --manifest-path src-tauri/Cargo.toml`、`npm run test:e2e`、`npm run tauri:build`、`git diff --check`
- [x] 实机 CRUD：新建/编辑/删除/恢复/永久删除，重启后状态一致（上轮 Orchestrator 已通过，本轮不重跑）。
- [x] RF8 release WebKitGTK 验收：三个 Select 的鼠标/键盘选择闭合、主题化展开、Bloom/Aurora 渲染均已通过（2026-08-21）。
- [x] 定向视觉复测：浅色 Bloom accent 与按钮/Switch、管理新版视觉、精简 Header、设置页平铺；RF5~RF7 通过。临时 WDIO 的 RF8 计算样式/截图不能替代 release 实机展开态。
- [x] E2E 覆盖真实删除→回收站→还原、永久删除确认，以及 K/J 的 `data-snippet-id` 选中变化。
- [x] 自动化覆盖新建、真实检索、敏感遮挡、双窗口路由；管理/设置导航和主题选择器已覆盖。
- [x] AC-08 敏感遮挡人工检查（a11y 另列为剩余人工 Gate）。
- [x] AC-11 快捷键失败回滚、AC-15 设置重启持久化、AC-18/19 回收站人工检查（上轮 Orchestrator 已通过，本轮不重跑）。
- [x] 浅/深/跟随系统：管理窗口自动化验证主题应用、后端持久化和 system 跟随。
- [x] reduced motion 人工检查。
- [x] 键盘焦点人工检查（沿用上次通过，冻结不重测）。
- [x] WCAG AA 人工检查（Orchestrator 使用隔离临时数据库通过，冻结不重测）。

## QA Result

- Status：`conditional_pass`
- Owner Back：`none`
- Verdict Date：2026-08-21
- Summary：release WebKitGTK RF8 复测通过。Manager sort 选择后关闭 ✅，Settings max-results/trash-auto-purge 主题化 DOM listbox 展开/闭合 ✅，键盘导航正常 ✅。RF5~RF7 及 Frozen Passed 项继续冻结。
- Coder Implementation：`snippetsApi.trashMove/trashRestore/trashPurgeOne` 已接现有 IPC；App 删除/还原/永久删除均走真实调用，成功刷新列表并保留错误 toast。
- Trash Wiring：删除调用 `trash_move`，回收站恢复调用 `trash_restore`；永久删除先显示主窗口 `role="alertdialog"`，仅确认按钮调用 `trash_purge_one`，取消不写库；回收站路径不使用 `notYet`。
- Confirmation：永久删除确认令牌由 App 生成并传给 `trashPurgeOne`，目标必须仍为已删除片段。
- Theme Evidence：统一 token 为 Bloom（浅色）/Aurora（深色）；当前 E2E 通过浅色 accent、按钮/Switch 计算样式及三主题同步断言。
- Manual RF5~RF8 Evidence（2026-08-21 release retest）：RF5 Header 贴边 ✅；RF6 Aurora 深色 danger 按钮可读 ✅；RF7 正文区重复说明已删除 ✅；RF8 release WebKitGTK 实机确认：Manager sort 选择后 listbox 关闭、标签更新为“名称首字母” ✅；Settings max-results/trash-auto-purge 展开为主题化 DOM listbox ✅；Bloom/Aurora 两主题渲染正确 ✅。截图见 `artifacts/e2e/spec13-rf8-release-*.png`。
- Checks：`npm run build` passed；`cargo test --manifest-path src-tauri/Cargo.toml` passed（55 passed, 0 failed；本轮未改 Rust）；`npm run test:e2e` passed（1 passing, 9.6s，含 RF8 Manager sort 回归）；`npm run tauri:build` passed；`git diff --check` passed。
- E2E Evidence：当前测试通过真实创建/检索、敏感 DTO 保护、主题与管理设置导航、双窗口键盘契约及真实回收站生命周期；K/J/ArrowDown 读取 `data-snippet-id`。
- Tab Cleanup：`QuickSearchWindow` 无 Tab 状态/处理/UI；旧 `quick-preview`、`quick-content-notice` CSS 与快捷键帮助条目已删除；当前 `src/` 与 E2E 无 `Tab`/`预览` 引用。QA 同步移除了 E2E 中已废止的 Tab/预览禁词断言；`SearchResultItem` 仍不携带正文。
- Scope Review：当前 diff 未修改 `src-tauri/**`、`src/types/snippet.ts`、数据库、领域类型、双窗口路由、AppMenu 或 astryx；`src/api/snippets.ts` 仅接入既有 trash IPC 命令。
- Confirmed Backend Regression（独立于 SPEC-13）：隔离临时数据库实机确认永久删除后快速搜索仍返回旧记录。根因是 Rust `SearchIndex` 只在 create/update/record_usage 时 upsert；`trash_move`/`trash_restore` 不更新 `deleted_at`，`trash_purge_one`/`trash_empty`/`auto_purge` 不移除或重建索引。不是临时数据库造成。由于 SPEC-13 禁止修改 `src-tauri/**`，登记为独立发布阻塞；不得重开已冻结的 CRUD，仅需后续验证回收站变更后的检索一致性。
- Orchestrator Results（上轮）：主题同步/切换、CRUD、敏感信息、设置重启持久化、快捷键失败回滚均已通过，本轮不要求重跑。
- Required Fix：已完成并通过 release 复测。ManagerWindow 排序 Select 移除外层原生 `<label>`，避免 option click 冒泡触发 trigger 重开；WDIO 回归断言覆盖闭合与焦点恢复；release WebKitGTK 三 Select Bloom/Aurora 闭合/展开/选择路径均已通过。
- Header：左侧仅软件图标；右侧仅快捷键帮助、主题切换、设置；Alt+O 仍由主窗口监听有效。
- Manual Pass：`prefers-reduced-motion`、WCAG AA、键盘焦点通过并冻结；快捷键回滚等 Frozen Passed 项不重跑。
- Environment Blocker：release 启动发现 `/home/ray/.local/share/io.searchis.desktop/searchis.db` 存在、`/home/ray/.config/io.searchis.desktop/database.key` 缺失。不得生成新 key 覆盖；恢复原 key，或使用隔离 XDG 临时目录做 UI 复测。该问题独立于 RF8。
- Retest Criteria：所有 RF8 release 复测已完成，无需进一步 retest。

### Orchestrator 定向视觉复测（2026-08-13，第二轮）

- Evidence：`artifacts/manual/spec-13/dark-header-danger-layout-failures.png`（SHA-256 `182b313a4ab5aa78576c6863a92ca6f671b3c20207b6cf53a285032e7e4d5da9`）
- Frozen Passed（禁止重复验收）：CRUD、敏感信息、设置持久化、快捷键失败回滚、Tab 功能删除、浅色强调色、主题同步均已由 Orchestrator 确认通过。Coder/QA 不得再次要求验证；只有 Master 明确记录对应路径在后续被修改时才能重新开启。
- Findings / Required Fixes：
  - RF5 Header 贴边：移除 `.manager-header-inner` 的居中最大宽度约束；品牌图标与右侧操作分别贴近 Header 左右内边距，宽屏不应收缩到 1600px 中央容器。
  - RF6 深色危险按钮：彻底删除/重置等危险操作在 Aurora 深色下使用明确的深色 danger surface、danger border 和可读前景色，不得沿用 Bloom 浅粉底；浅色危险态同时保持可读。
  - RF7 删除重复说明：删除正文区“正文只保存在本地加密数据库。”文案，保留“文本内容”标签与字符数，避免主题切换/布局时重叠。
  - RF8 Select 视觉：最大结果数、回收站自动清理不再呈现浏览器默认方形 select；用现有 CSS/native select 做统一圆角、主题背景/边框、箭头、hover/focus-visible，保持原生键盘与无障碍语义，不引入新依赖。
- Retest Criteria：自动门槛通过后，Orchestrator 只复测 RF8 三个 Select 的闭合态与展开态；其余冻结项目不重测。

## 完成定义

新版管理/设置视觉可观察，所有领域事务与系统状态契约无回归；QA `passed`，状态回写完成。

## Orchestrator Gate（2026-08-21）

- 结论：RF8 release 实机确认通过（三个 Select 闭合/展开/选择路径、Bloom/Aurora 渲染正常），人工 Gate 无具体失败。
- 处置：`conditional_pass` 成立，SPEC-13 由 `in_qa` 收为 `done`；Frozen Passed 项不重测。Code 冻结，放行 SPEC-14。
