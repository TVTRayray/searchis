# SPEC-12 快速检索窗口视觉重构并入

## 基本信息

- 当前状态：`in_qa`（自动验收 conditional_pass，等待 Orchestrator 人工 Gate）
- 当前责任角色：`Orchestrator`（仅回报人工验收结果；Coder/QA 停止循环）
- 关联 PRD：`FR-SCH-01~04`、`FR-PCK-01~04`、`AC-01/03/04/05/08/12/13/14/20`、`NFR-PERF-01~03`、`NFR 11.4`
- 前置 Spec：`SPEC-02/03/11`（done）
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 允许修改：`package*.json`、`components.json`、`public/favicon.svg`、`src/assets/fonts/**`、`src/lib/utils.ts`、`src/components/ui/**`、`src/components/layout/**`、`src/components/AppShell.tsx`、`src/components/ThemeProvider.tsx`、`src/components/QuickSearchWindow.tsx`、`src/index.css`、`e2e/**`、`wdio.conf.ts`，以及删除不再引用的对应 astryx 原语
- 禁止修改：`src-tauri/**`、存储/检索/粘贴契约、`src/api/snippets.ts`、`src/types/snippet.ts`、`src/App.tsx` 的双窗口路由、`docs/prds/**`

## 目标

把外部重构版的 shadcn/ui + layout 视觉系统和 QuickSearchWindow 外观定向移植到当前正式应用，同时完整保留已经验收的独立无边框检索窗口、真实数据库检索、复制/自动粘贴、跨窗口跳转和键盘语义。

## 成功标准

- [x] 快速检索窗口采用外部重构版 cmdk 分组、Key 高亮、元信息预览、空态建议、Aurora/Bloom 主题和本地字体。
- [x] 仍由独立 Tauri `search` 窗口承载，不退回主窗口内模态层。
- [x] 数据仍来自 `snippetsApi.search/executePaste/prepareNew`；不得引入 `initialSnippets`、`searchEngine` 或 localStorage 片段真相。
- [x] Alt+O、Esc、Enter、Ctrl+Enter、Ctrl+1~9、Ctrl+E、Ctrl+N、Tab、方向键与输入法组合态行为不变。
- [x] 敏感结果 DTO 不含正文，默认可见文本和 a11y label 不泄露正文。
- [x] `prefers-reduced-motion` 生效；浅/深主题关键文本满足 WCAG AA。

## 并入规则

1. 只移植视觉结构、样式、字体、图标和 UI 原语；业务逻辑按当前项目实现保留。
2. 禁止复制外部的 `src/App.tsx`、`src/api/**`、`src/types/**`、`src/data/**`、`src/utils/searchEngine.ts`、`src-tauri/**`。
3. 新依赖仅允许外部已使用且本 slice 实际消费的 `class-variance-authority`、`clsx`、`cmdk`、`radix-ui`、`sonner`、`tailwind-merge`、`tw-animate-css`；未使用依赖不得加入。
4. 先保留 astryx，QuickSearchWindow 切换完成且验证通过后，只删除其不再引用的原语；最终清理由 SPEC-14 完成。

## 失败场景

| 场景 | 预期 |
|---|---|
| 视觉组件要求 mock 类型/数据 | 适配到当前 SearchResultItem，不复制 mock |
| 新 cmdk 键盘处理覆盖现有快捷键 | 保留当前手工键盘契约，以回归测试为准 |
| Tauri IPC 失败 | 保留当前错误提示，窗口不崩溃、不假报成功 |
| 主题/动效不兼容 | 回滚本 slice，不影响后端与窗口架构 |

## 验收

- [x] `npm run build`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`（55 passed）
- [x] `npm run test:e2e`（1 passed；按窗口 label 定向操作，覆盖创建、分组检索、Tab、方向键/J/K、IME、Ctrl+Enter、Ctrl+1、Ctrl+E、Ctrl+N、真实 prepare_new 校验失败、敏感正文不泄露与 Esc）
- [ ] Orchestrator 人工 Gate A：Alt+O 呼出独立窗口，完整检索/复制/粘贴/关闭/跨窗口流程。（涉及真实前台输入和剪贴板，Coder/QA 禁止自行自动化）
- [ ] Orchestrator 人工 Gate B：浅/深主题、reduced motion、键盘焦点与基本 a11y 目测。

## QA Result

- Status：`conditional_pass`
- Owner Back：`none`（等待 Orchestrator 人工 Gate，不退回 Coder）
- Checks：Coder 已记录 `npm run build`、55 个 Rust tests、external/embedded E2E 和 `npm run tauri:build` 通过；QA 复核当前 spec、diff、既有测试与验收证据。
- Findings：未发现已证实的业务回归或禁止范围修改；没有可交给 Coder 的代码修复项。
- Risks：E2E 的 DOM/合成键盘事件不能替代目标桌面的真实前台键盘、剪贴板或 X11 Ctrl+V。
- Missing Tests：仅剩上方 Orchestrator 人工 Gate A/B。
- Required Fixes：`none`。人工 Gate 缺证据不是代码缺陷，不得再次退回 Coder。
- Closure Rule：Orchestrator 回报 A/B 均通过后，QA 将 Status 改为 `passed` 并关闭 SPEC-12；若失败，必须记录具体步骤、期望/实际结果和截图/错误，届时才创建单次明确 Required Fixes 退回 Coder。

## 完成定义

用户可观察到新版快速检索视觉，所有既有真实数据、双窗口和键盘行为保持通过；QA `passed`，状态回写完成。
