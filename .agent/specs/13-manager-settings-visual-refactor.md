# SPEC-13 管理窗口与设置视觉重构并入

## 基本信息

- 当前状态：`todo`
- 当前责任角色：`Coder`
- 关联 PRD：`FR-SNP-01~04`、`FR-MGT-01~05`、`FR-SET-01/02`、`AC-02/06/07/08/11/15/18/19`
- 前置 Spec：`SPEC-12`（done）
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 允许修改：`src/App.tsx` 的主窗口渲染层、`src/components/HeaderBar.tsx`、`ManagerWindow.tsx`、`SettingsModal.tsx`、本 slice 实际使用的 `src/components/ui/**` 与样式
- 禁止修改：`src-tauri/**`、`src/api/snippets.ts` 的 IPC 契约、`src/types/snippet.ts` 领域字段、数据库/设置事务、双窗口路由、`docs/prds/**`

## 目标

把外部重构版的 HeaderBar、ManagerWindow、SettingsModal 外观并入正式主窗口，同时保留真实 CRUD、筛选排序、回收站、敏感遮挡、revision 冲突、系统设置和 XDG/KGlobalAccel 实际状态。

## 成功标准

- [ ] 管理页使用新版导航、列表、表单、卡片、dialog/toast 和主题样式。
- [ ] 全部/置顶/最近使用/标签/回收站视图与四种排序仍走当前真实后端。
- [ ] 新建、编辑、删除、还原、永久删除、清空、敏感显隐和 revision 冲突保持原行为。
- [ ] 设置仍由 `settingsApi` 与系统真实状态驱动；不得使用 external App 的 localStorage 配置/片段逻辑。
- [ ] 快捷键保持 Linux/KDE：Alt+O、Ctrl+Enter、Ctrl+V；不得出现 Option/Cmd/⌘。
- [ ] 不展示或保存 `playAudioFeedback`；`restoreClipboard` 保持 false 且无开关。
- [ ] 成功提示节制、错误明确；日志/a11y 不含正文。

## 并入规则

1. 外部 `App.tsx` 只作为视觉参考，禁止整文件覆盖；保留当前窗口标签路由、真实 API 状态与错误处理。
2. 外部 mock CRUD/localStorage/初始数据/假系统状态一律不并入。
3. 组件 props 应适配当前 PersistedSnippet/Settings 契约，不反向修改领域类型迎合视觉组件。
4. 本 slice 不处理 OnboardingWizard/KeyboardShortcutsModal，留给 SPEC-14。

## 验收

- [ ] `npm run build`、`cargo test`、`npm run test:e2e`
- [ ] 实机 CRUD：新建/编辑/删除/恢复/永久删除，重启后状态一致。
- [ ] AC-08 敏感遮挡与 a11y 检查。
- [ ] AC-11 快捷键失败回滚、AC-15 设置重启持久化、AC-18/19 回收站检查。
- [ ] 浅/深/跟随系统 + reduced motion + WCAG AA。

## QA Result

- Status：`not_run`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

新版管理/设置视觉可观察，所有领域事务与系统状态契约无回归；QA `passed`，状态回写完成。
