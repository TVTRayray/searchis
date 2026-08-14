# SPEC-13 管理窗口与设置视觉重构并入

## 基本信息

- 当前状态：`qa_failed`（第三轮 release 实机复测：RF8 仍失败）
- 当前责任角色：`Coder`
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
- [ ] Coder 修复 RF8：release 实机 WebKitGTK 中两个 Select 的控件及展开选项仍未匹配 Bloom/Aurora 主题与新版样式。
- [x] 定向视觉复测：浅色 Bloom accent 与按钮/Switch、管理新版视觉、精简 Header、设置页平铺；RF5~RF7 通过。临时 WDIO 的 RF8 计算样式/截图不能替代 release 实机展开态。
- [x] E2E 覆盖真实删除→回收站→还原、永久删除确认，以及 K/J 的 `data-snippet-id` 选中变化。
- [x] 自动化覆盖新建、真实检索、敏感遮挡、双窗口路由；管理/设置导航和主题选择器已覆盖。
- [x] AC-08 敏感遮挡人工检查（a11y 另列为剩余人工 Gate）。
- [x] AC-11 快捷键失败回滚、AC-15 设置重启持久化、AC-18/19 回收站人工检查（上轮 Orchestrator 已通过，本轮不重跑）。
- [x] 浅/深/跟随系统：管理窗口自动化验证主题应用、后端持久化和 system 跟随。
- [x] reduced motion 人工检查。
- [x] 键盘焦点人工检查（沿用上次通过，冻结不重测）。
- [ ] WCAG AA 人工检查（RF8 修复后仅检查相关 Select 可读性）。

## QA Result

- Status：`failed`（第三轮 release 实机复测）
- Owner Back：`Coder`
- Verdict Date：2026-08-13
- Summary：自动检查与 RF5~RF7 通过；release 实机确认 RF8 的两个 Select 仍未满足 Bloom/Aurora 主题与新版样式，退回 Coder。Reduced motion 与键盘焦点已通过并冻结。
- Coder Implementation：`snippetsApi.trashMove/trashRestore/trashPurgeOne` 已接现有 IPC；App 删除/还原/永久删除均走真实调用，成功刷新列表并保留错误 toast。
- Trash Wiring：删除调用 `trash_move`，回收站恢复调用 `trash_restore`；永久删除先显示主窗口 `role="alertdialog"`，仅确认按钮调用 `trash_purge_one`，取消不写库；回收站路径不使用 `notYet`。
- Confirmation：永久删除确认令牌由 App 生成并传给 `trashPurgeOne`，目标必须仍为已删除片段。
- Theme Evidence：统一 token 为 Bloom（浅色）/Aurora（深色）；当前 E2E 通过浅色 accent、按钮/Switch 计算样式及三主题同步断言。
- Manual RF5~RF8 Evidence（2026-08-14）：Header 贴边、Aurora 深色危险按钮、正文区重复说明删除通过；临时 WDIO 对 RF8 的 computed style 与静态截图为假阳性，release 实机展开 Select 后仍呈原生/错误主题样式。
- Checks：`npm run build` passed；`cargo test --manifest-path src-tauri/Cargo.toml` passed（55 passed, 0 failed）；`npm run test:e2e` passed（1 passing, 10.6s）；`npm run tauri:build` passed；`git diff --check` passed。
- E2E Evidence：当前测试通过真实创建/检索、敏感 DTO 保护、主题与管理设置导航、双窗口键盘契约及真实回收站生命周期；K/J/ArrowDown 读取 `data-snippet-id`。
- Tab Cleanup：`QuickSearchWindow` 无 Tab 状态/处理/UI；旧 `quick-preview`、`quick-content-notice` CSS 与快捷键帮助条目已删除；当前 `src/` 与 E2E 无 `Tab`/`预览` 引用。QA 同步移除了 E2E 中已废止的 Tab/预览禁词断言；`SearchResultItem` 仍不携带正文。
- Scope Review：当前 diff 未修改 `src-tauri/**`、`src/types/snippet.ts`、数据库、领域类型、双窗口路由、AppMenu 或 astryx；`src/api/snippets.ts` 仅接入既有 trash IPC 命令。
- Known Backend Limitation：被禁止修改的 Rust 检索索引在 `trash_move` 后不会即时更新；回收站 E2E 使用真实 `snippet_list` 验证持久状态，不以旧索引作为 Required Fix 证明。
- Orchestrator Results（上轮）：主题同步/切换、CRUD、敏感信息、设置重启持久化、快捷键失败回滚均已通过，本轮不要求重跑。
- Required Fix：RF8 重新打开。必须在 release WebKitGTK 实际展开“最大结果数”和“回收站自动清理”验证控件与选项层；仅断言闭合态 computed style 不足。若原生 `<select>` 展开层无法可靠主题化，使用仓库现有能力实现最小的可访问 Select，不以增加新依赖作为默认方案。
- Header：左侧仅软件图标；右侧仅快捷键帮助、主题切换、设置；Alt+O 仍由主窗口监听有效。
- Manual Pass：`prefers-reduced-motion`、键盘焦点通过并冻结；快捷键回滚等 Frozen Passed 项不重跑。
- Environment Blocker：release 启动发现 `/home/ray/.local/share/io.searchis.desktop/searchis.db` 存在、`/home/ray/.config/io.searchis.desktop/database.key` 缺失。不得生成新 key 覆盖；恢复原 key，或使用隔离 XDG 临时目录做 UI 复测。该问题独立于 RF8。
- Retest Criteria：Coder 修复 RF8 并完成自动门槛后，Orchestrator 仅复测两个 Select 的 Bloom/Aurora 闭合态、展开态及相关 WCAG 可读性；其他项目禁止重测。

### Orchestrator 定向视觉复测（2026-08-13，第二轮）

- Evidence：`artifacts/manual/spec-13/dark-header-danger-layout-failures.png`（SHA-256 `182b313a4ab5aa78576c6863a92ca6f671b3c20207b6cf53a285032e7e4d5da9`）
- Frozen Passed（禁止重复验收）：CRUD、敏感信息、设置持久化、快捷键失败回滚、Tab 功能删除、浅色强调色、主题同步均已由 Orchestrator 确认通过。Coder/QA 不得再次要求验证；只有 Master 明确记录对应路径在后续被修改时才能重新开启。
- Findings / Required Fixes：
  - RF5 Header 贴边：移除 `.manager-header-inner` 的居中最大宽度约束；品牌图标与右侧操作分别贴近 Header 左右内边距，宽屏不应收缩到 1600px 中央容器。
  - RF6 深色危险按钮：彻底删除/重置等危险操作在 Aurora 深色下使用明确的深色 danger surface、danger border 和可读前景色，不得沿用 Bloom 浅粉底；浅色危险态同时保持可读。
  - RF7 删除重复说明：删除正文区“正文只保存在本地加密数据库。”文案，保留“文本内容”标签与字符数，避免主题切换/布局时重叠。
  - RF8 Select 视觉：最大结果数、回收站自动清理不再呈现浏览器默认方形 select；用现有 CSS/native select 做统一圆角、主题背景/边框、箭头、hover/focus-visible，保持原生键盘与无障碍语义，不引入新依赖。
- Retest Criteria：自动门槛通过后，Orchestrator 只复测 RF5~RF8、reduced-motion、WCAG/键盘焦点；其余路径不重跑。

## 完成定义

新版管理/设置视觉可观察，所有领域事务与系统状态契约无回归；QA `passed`，状态回写完成。
