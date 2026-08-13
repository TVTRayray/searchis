# SPEC-13 管理窗口与设置视觉重构并入

## 基本信息

- 当前状态：`qa_failed`（Orchestrator 人工视觉验收失败）
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
- [x] 新建/编辑表单、敏感显隐、复制/粘贴和 revision 请求边界保持原行为；回收站已接通真实 IPC。
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
- [x] E2E 覆盖真实删除→回收站→还原、永久删除确认，以及 K/J 的 `data-snippet-id` 选中变化。
- [x] 自动化覆盖新建、真实检索、敏感遮挡、双窗口路由；管理/设置导航和主题选择器已覆盖。
- [ ] AC-08 敏感遮挡与 a11y 人工检查。
- [ ] AC-11 快捷键失败回滚、AC-15 设置重启持久化、AC-18/19 回收站人工检查。
- [x] 浅/深/跟随系统：管理窗口自动化验证主题应用、后端持久化和 system 跟随。
- [ ] reduced motion + WCAG AA 人工检查。

## QA Result

- Status：`failed`（Orchestrator 人工验收，2026-08-13）
- Coder Implementation：新增 `snippetsApi.trashMove/trashRestore/trashPurgeOne`；App 删除/还原/永久删除改走真实 IPC，成功刷新列表并保留错误 toast。
- Confirmation：永久删除使用主窗口内 `role="alertdialog"`，仅点击“确认永久删除”后调用 `trash_purge_one`；取消不写库。
- Theme Evidence：保留既有管理设置页浅色、深色、system 自动化证据。
- Checks：`npm run build` passed；`cargo test --manifest-path src-tauri/Cargo.toml` 55 passed；`npm run test:e2e` passed（1 test，8.5s）；`npm run tauri:build` passed；`git diff --check` passed。
- E2E Evidence：真实删除→回收站→还原、删除→确认永久删除、真实列表状态校验；K/J/ArrowDown 统一读取 `data-snippet-id`。
- Scope Review：未修改 `src-tauri/**`、`src/types/snippet.ts`、数据库、领域类型、双窗口路由或 astryx；`src/api/snippets.ts` 仅新增现有命令调用。
- Known Backend Limitation：被禁止修改的 Rust 检索索引在 `trash_move` 后不会即时更新；回收站 E2E 使用真实 `snippet_list` 验证持久状态，不以旧索引作为 Required Fix 证明。
- Orchestrator Results：主题同步/切换通过；CRUD 通过；敏感信息通过；设置重启持久化通过；快捷键修改在目标机因 KGlobalAccel D-Bus 不可用而正确报错（需保留旧值并由后续复测确认）。
- Evidence：
  - `artifacts/manual/spec-13/light-theme-inverse-actions.png`（浅色模式四处反主题强调按钮）
  - `artifacts/manual/spec-13/light-theme-inverse-switches.png`（浅色模式开关使用近黑色强调态）
- Findings：
  1. 浅色主题的新增、呼出快速窗口、快速粘贴、保存和 Switch 使用近黑色 accent，形成明显反主题色；根因是浅色主题 accent token 策略，不得逐个硬编码覆盖。
  2. 管理窗口仍明显沿用旧 Astryx 中性视觉，没有达到与 SPEC-12 Aurora/Bloom 新视觉体系一致的管理主题；SPEC-13 的“新版管理/设置视觉可观察”未满足。
  3. Header 信息冗余：Orchestrator 要求左侧仅软件图标，移除中间标签页导航；右侧仅重排快捷键帮助、主题切换、设置。快速检索入口由既有全局快捷键/其他入口保留。
  4. 设置页仍为页面内再嵌套带标题栏卡片；要求移除二次标题容器，设置内容直接铺在设置页面。
- Required Fixes：统一调整浅/深强调色 token；完成管理/设置新视觉而非仅主题变量换色；按上述要求精简 Header 与平铺设置页；**彻底删除快速检索 Tab 元信息功能**（状态、键盘处理、面板、CSS、footer/帮助文案及 E2E），保留“搜索 DTO 永不携带正文”的安全约束；补浅色视觉/对比度 E2E 或可计算断言并复跑现有门槛。
- Retest Criteria：Coder/QA 自动门槛通过后，Orchestrator 只复测四项视觉 finding、快捷键失败后旧值回滚、reduced-motion/WCAG；CRUD/敏感/设置重启已通过无需重跑，除非相关路径被改动。

## 完成定义

新版管理/设置视觉可观察，所有领域事务与系统状态契约无回归；QA `passed`，状态回写完成。
