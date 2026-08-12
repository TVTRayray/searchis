# SPEC-12 快速检索窗口视觉重构并入

## 基本信息

- 当前状态：`qa_failed`（Master 审计：流程与行为回归，退回 Coder）
- 当前责任角色：`Coder`
- 关联 PRD：`FR-SCH-01~04`、`FR-PCK-01~04`、`AC-01/03/04/05/08/12/13/14/20`、`NFR-PERF-01~03`、`NFR 11.4`
- 前置 Spec：`SPEC-02/03/11`（done）
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 允许修改：`package*.json`、`components.json`、`public/favicon.svg`、`src/assets/fonts/**`、`src/lib/utils.ts`、`src/components/ui/**`、`src/components/layout/**`、`src/components/AppShell.tsx`、`src/components/ThemeProvider.tsx`、`src/components/QuickSearchWindow.tsx`、`src/index.css`，以及删除不再引用的对应 astryx 原语
- 禁止修改：`src-tauri/**`、存储/检索/粘贴契约、`src/api/snippets.ts`、`src/types/snippet.ts`、`src/App.tsx` 的双窗口路由、`docs/prds/**`

## 目标

把外部重构版的 shadcn/ui + layout 视觉系统和 QuickSearchWindow 外观定向移植到当前正式应用，同时完整保留已经验收的独立无边框检索窗口、真实数据库检索、复制/自动粘贴、跨窗口跳转和键盘语义。

## 成功标准

- [ ] 快速检索窗口采用外部重构版 cmdk 分组、Key 高亮、元信息预览、空态建议、Aurora/Bloom 主题和本地字体。
- [ ] 仍由独立 Tauri `search` 窗口承载，不退回主窗口内模态层。
- [ ] 数据仍来自 `snippetsApi.search/executePaste/prepareNew`；不得引入 `initialSnippets`、`searchEngine` 或 localStorage 片段真相。
- [ ] Alt+O、Esc、Enter、Ctrl+Enter、Ctrl+1~9、Ctrl+E、Ctrl+N、Tab、方向键与输入法组合态行为不变。
- [ ] 敏感结果 DTO 不含正文，默认可见文本和 a11y label 不泄露正文。
- [ ] `prefers-reduced-motion` 生效；浅/深主题关键文本满足 WCAG AA。

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

- [ ] `npm run build`
- [ ] `cargo test`（证明后端无回归）
- [ ] `npm run test:e2e`
- [ ] 实机：Alt+O 呼出独立窗口，完整检索/复制/粘贴/关闭/跨窗口流程。
- [ ] 两主题 + reduced motion + 键盘/a11y 目测。

## QA Result

- Status：`failed`（2026-08-12 Master 复核）
- Findings：
  - F1：原 `匹配片段结果 / 展示 N / 总数 M` 表头被删除；`total` 仍被读取但未展示，违反 FR-SCH-03，用户已实机确认表头缺失。
  - F2：Ctrl+Enter 调用 `executePaste(..., true)` 并关闭窗口，与“仅复制并保留窗口”契约相反。
  - F3：Ctrl+N/空态新建直接传原始 query，绕过 `prepareNew` 规范化，AC-13 `work addr -> work-addr` 可能回归。
  - F4：移除了窗口重新显示时清空查询并聚焦的 `onFocusChanged` 逻辑，只保留首次 mount autofocus。
  - F5：SPEC-12 与 SPEC-13/14 被合并为单个 commit，未按顺序独立验收；本 spec 11 个验收项均未勾选却标记 done。
- Risks：cmdk 全局 capture 键盘处理改变了已验收快捷键语义；E2E 仍失败，无法证明无回归。
- Missing Tests：`npm run test:e2e` 未通过；无实机两主题/reduced-motion/IME/a11y 记录；QA 未执行。
- Required Fixes：恢复 FR-SCH-03 表头；恢复 Ctrl+Enter/Enter/Ctrl+N/窗口重开契约；补齐并记录本 spec 全部验证；不得顺带修 SPEC-13/14。
- Retest Criteria：build + cargo test + 可用 E2E/等价可靠 UI 自动化 + 实机独立搜索窗口完整键盘流程；QA `passed` 后才能转 done。

## 完成定义

用户可观察到新版快速检索视觉，所有既有真实数据、双窗口和键盘行为保持通过；QA `passed`，状态回写完成。
