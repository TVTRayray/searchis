# SPEC-12 快速检索窗口视觉重构并入

## 基本信息

- 当前状态：`done`
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

- Status：`not_run`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

用户可观察到新版快速检索视觉，所有既有真实数据、双窗口和键盘行为保持通过；QA `passed`，状态回写完成。
