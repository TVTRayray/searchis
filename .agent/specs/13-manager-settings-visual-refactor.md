# SPEC-13 管理窗口与设置视觉重构并入

## 基本信息

- 当前状态：`qa_failed`（真实回收站前端接线缺失，自动化断言不足）
- 当前责任角色：`Coder`
- 关联 PRD：`FR-SNP-01~04`、`FR-MGT-01~05`、`FR-SET-01/02`、`AC-02/06/07/08/11/15/18/19`
- 前置 Spec：`SPEC-12`（done）
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 允许修改：`src/App.tsx` 的主窗口渲染层、`src/components/HeaderBar.tsx`、`ManagerWindow.tsx`、`SettingsModal.tsx`、本 slice 实际使用的 `src/components/ui/**` 与样式
- 禁止修改：`src-tauri/**`、`src/api/snippets.ts` 的 IPC 契约、`src/types/snippet.ts` 领域字段、数据库/设置事务、双窗口路由、`docs/prds/**`

## 目标

把外部重构版的 HeaderBar、ManagerWindow、SettingsModal 外观并入正式主窗口，同时保留真实 CRUD、筛选排序、回收站、敏感遮挡、revision 冲突、系统设置和 XDG/KGlobalAccel 实际状态。

## 成功标准

- [x] 管理页使用新版导航、列表、表单、卡片、toast 和主题样式；未迁移 Onboarding/KeyboardShortcuts。
- [x] 全部/置顶/最近使用/标签/回收站视图与四种排序的既有本地筛选逻辑保持不变。
- [x] 新建/编辑表单、敏感显隐、复制/粘贴和 revision 请求边界保持原行为；回收站回调未改写。
- [x] 设置仍由 `settingsApi` 与系统真实状态驱动；未引入 external App 的 localStorage 配置/片段逻辑。
- [x] 快捷键保持 Linux/KDE：Alt+O、Ctrl+Enter、Ctrl+V；目标迁移文件不含 Option/Cmd/⌘。
- [x] 不展示或保存 `playAudioFeedback`；`restoreClipboard` 保持 false 且无可操作开关。
- [x] 成功提示节制、错误明确；新增管理/设置默认文本不含正文。

## 并入规则

1. 外部 `App.tsx` 只作为视觉参考，禁止整文件覆盖；保留当前窗口标签路由、真实 API 状态与错误处理。
2. 外部 mock CRUD/localStorage/初始数据/假系统状态一律不并入。
3. 组件 props 应适配当前 PersistedSnippet/Settings 契约，不反向修改领域类型迎合视觉组件。
4. 本 slice 不处理 OnboardingWizard/KeyboardShortcutsModal，留给 SPEC-14。

## 验收

- [x] `npm run build`、`cargo test --manifest-path src-tauri/Cargo.toml`、`npm run test:e2e`、`npm run tauri:build`、`git diff --check`
- [ ] 实机 CRUD：新建/编辑/删除/恢复/永久删除，重启后状态一致。
- [x] 自动化覆盖新建、真实检索、敏感遮挡、双窗口路由；管理/设置导航和主题选择器已覆盖。
- [ ] AC-08 敏感遮挡与 a11y 人工检查。
- [ ] AC-11 快捷键失败回滚、AC-15 设置重启持久化、AC-18/19 回收站人工检查。
- [x] 浅/深/跟随系统：管理窗口自动化验证主题应用、后端持久化和 system 跟随。
- [ ] reduced motion + WCAG AA 人工检查。

## QA Result

- Status：`failed`（Master 复核：存在确定无法通过的成功标准，不进入人工验收）
- Coder Implementation：`HeaderBar`、`ManagerWindow`、`SettingsModal` 和 `App.tsx` 主窗口壳迁移到语义主题变量；未修改 astryx、Rust、API、领域类型或双窗口路由。
- Theme Evidence：E2E 验证管理设置页浅色、深色、system 选择，检查 `data-theme`、后端 `settings_get` 持久化及 system 跟随；设置页无 `playAudioFeedback`、Option/Cmd/⌘ 文案。
- Checks（QA 本轮复跑）：`npm run build` passed（1682 modules）；`cargo test --manifest-path src-tauri/Cargo.toml` 55 passed；`npm run test:e2e` passed（1 test，7.8s）；`npm run tauri:build` passed；`git diff --check` passed。
- Scope Review：仅修改 SPEC-13 白名单文件；未修改 `src-tauri/**`、`src/api/snippets.ts`、`src/types/snippet.ts`、`src/components/astryx/**` 或双窗口路由。
- Findings / Risks：
  1. 当前 `App.tsx` 的 `onDeleteSnippet`、`onRestoreSnippet`、`onPermanentDeleteSnippet` 仍绑定 `notYet('回收站')`，`src/api/snippets.ts` 也没有对应 trash API。该问题未由本次视觉 diff 引入，但若 SPEC-13 要求 AC-06/07/18/19 的用户可观察闭环，人工验收将失败，需由 Orchestrator/Master 决定纳入后续修复还是沿用既有范围边界。
  2. 当前自动化未覆盖真实 CRUD 编辑/删除/恢复/永久删除、重启后设置持久化、AC-08 a11y、AC-11 快捷键失败回滚、AC-18/19 回收站确认与事务回滚、reduced-motion/WCAG AA。
  3. 现有 E2E 的 K/J 等待断言仍使用 `data-value`，而选中项身份使用 `data-snippet-id`；即使测试通过，也不能充分证明 K/J 真的移动了选中项。
- Required Fixes：
  1. 接通既有 Rust `trash_move`、`trash_restore`、`trash_purge_one`（及当前 UI 已提供入口对应的 `trash_empty`）到前端 API 与 `App.tsx` 回调，刷新真实列表并保留现有错误提示/确认语义；不得修改 Rust、数据库或领域类型。
  2. E2E 覆盖真实删除→回收站→还原，以及永久删除确认路径；K/J 移动断言统一使用稳定 `data-snippet-id`。
  3. 修复后复跑 build、55 Rust tests、E2E、tauri:build、diff check，再交 QA。
- Deferred Manual Checks：自动门槛通过后，Orchestrator 再复测主题、reduced-motion、WCAG/a11y、CRUD、设置重启持久化、快捷键失败回滚和回收站路径。

## 完成定义

新版管理/设置视觉可观察，所有领域事务与系统状态契约无回归；QA `passed`，状态回写完成。
