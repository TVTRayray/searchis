# SPEC-12 快速检索窗口视觉重构并入

## 基本信息

- 当前状态：`qa_failed`（Orchestrator 人工 Gate 发现 3 个具体缺陷）
- 当前责任角色：`Coder`（一次性修复 RF1~RF3；禁止扩大范围）
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
- [x] Orchestrator 人工 Gate A：Alt+O 呼出独立窗口，完整检索/复制/粘贴/关闭/跨窗口流程。（2026-08-12 通过）
- [ ] Orchestrator 人工 Gate B：主题切换控件通过；独立窗口主题、窗口贴合与失焦关闭失败，见 QA Findings。

## QA Result

- Status：`failed`（Orchestrator 人工 Gate，2026-08-12）
- Owner Back：`Coder`
- Checks：Gate A 全部通过；Gate B 的管理窗口深/浅切换成功，但独立检索窗口实机失败。
- Evidence：`artifacts/manual/spec-12/search-window-theme-frame-failure.png`（SHA-256 `a4faa04f68e598e7642cd3caa7d7e0b5fd91d24b7a0844b257405068dbee712a`）。
- Findings：
  - F1 / Theme sync：管理窗口切到浅色后，独立 `search` WebView 仍显示深色。当前每个窗口各自持有 ThemeProvider 状态，检索窗口只读取自身 localStorage 初始化值，不在呼出/聚焦时从真实 `settingsApi.get()` 同步主题。
  - F2 / Window fit：截图中 Tauri 窗口外层约 780×916，实际检索面板只占顶部约 256px，剩余 WebView 是深色空壳；预期窗口客户区由真实检索面板完整填满，只有面板自身一圈实际边框，不得出现外层背景/边框。检查 `.quick-search-shell`、`html/body/#root` 高度与窗口实际尺寸/缩放，不得仅用额外装饰层遮盖。
  - F3 / Blur close：点击其他窗口后检索窗口保持可见。当前 `onFocusChanged` 对 `payload=false` 直接 return；预期独立检索窗口失焦立即调用既有 `onClose/windowApi.closeSearch` 隐藏。
- Required Fixes：
  - RF1：独立窗口每次呼出/重新获得焦点时从后端 settings 真相应用有效主题；不得新增 localStorage 领域真相或修改 `src/api/snippets.ts`。
  - RF2：修正根节点与检索壳尺寸，使 780×500 客户区由检索 UI 正确填满，内部边框贴合实际窗口；保留无边框、置顶、独立窗口形态。
  - RF3：失焦关闭；避免由自身关闭、Ctrl+E/Ctrl+N 跨窗口切换造成重复调用或破坏 Gate A 行为。
- Retest Criteria：自动化继续全绿；新增/更新可自动验证的主题加载、尺寸和 blur 关闭断言；Orchestrator 只复测 RF1~RF3，Gate A 已通过部分无需重跑，除非修改触及对应路径。

## 完成定义

用户可观察到新版快速检索视觉，所有既有真实数据、双窗口和键盘行为保持通过；QA `passed`，状态回写完成。
