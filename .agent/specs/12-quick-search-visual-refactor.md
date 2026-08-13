# SPEC-12 快速检索窗口视觉重构并入

## 基本信息

- 当前状态：`done`
- 当前责任角色：`QA`
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
- [x] `npm run test:e2e`（1 passed；按窗口 label 定向操作，覆盖创建、分组检索、Tab、方向键/J/K、IME、Ctrl+Enter、Ctrl+1、Ctrl+E、Ctrl+N、真实 prepare_new 校验失败、敏感正文不泄露、主题计算明度、窗口尺寸、失焦关闭、搜索栏 focus 样式与 Esc）
- [x] Orchestrator 人工 Gate A：Alt+O 呼出独立窗口，完整检索/复制/粘贴/关闭/跨窗口流程。（2026-08-12 通过）
- [x] RF2：检索内容填满 780×500 客户区，无外层壳/额外边框。（Orchestrator 实机通过）
- [x] RF3：鼠标点其他窗口，检索窗口立即失焦关闭。（Orchestrator 实机通过）
- [x] RF1：检索窗口主题同步与搜索栏 focus 边线。（Orchestrator 实机通过）

## QA Result

- Status：`passed`
- QA 本轮复跑：`npm run build` passed（1682 modules）；`cargo test --manifest-path src-tauri/Cargo.toml` 55 passed；`npm run test:e2e` passed（1 test，6s，无 selected-navigation/no-such-window/mock-store 警告）；`npm run tauri:build` passed；`git diff --check` passed。
- Coder Fix Review：`CommandItem` 增加稳定 `data-snippet-id`；E2E 选中项改读业务 ID并从主窗口交叉查询；RF3 失焦断言移到测试末尾，隐藏 search 窗口后不再继续调用 search context。
- RF1：自动化已验证主题明度、浅/深主题切换、focus 样式和窗口时序；Orchestrator 已实机确认深色主题同步及搜索栏 focus 边线通过。
- RF2/RF3：Orchestrator 已实机通过；Gate A 已于 2026-08-12 通过，不重跑。
- Scope Review：未触及禁止的 Rust、IPC、存储、API、类型或双窗口路由；本轮未操作 KDE 全局快捷键、系统剪贴板、自动粘贴或真实前台键盘。
- Findings：无未解决的 SPEC-12 范围缺陷。

### SPEC-12 Required Fixes

- [x] 选中项使用稳定的 `data-snippet-id`，并与主窗口真实后端结果交叉校验。
- [x] RF3 生命周期断言移至测试末尾；隐藏 search 窗口后不再调用 search WebView；QA 当前复跑无生命周期竞态。
- [x] Orchestrator 已实机复测 RF1 深色主题和搜索栏 focus 边线。

### Orchestrator 反馈中的超范围项（非本 spec，需记入后续 spec）

- **#1 管理界面主题未同步** → 归入 SPEC-13（管理窗口视觉重构）
- **#4 设置页面内层容器多余** → 归入 SPEC-13
- **#5 软件无预设圆角效果** → 归入 SPEC-13（全局主题）
- **#6 标题栏内容冗余** → 归入 SPEC-13
- **#7 全局菜单栏未显示** → 归入 SPEC-09 回归检查（SPEC-09 已 done，需确认当前是否退化）

### Retest Criteria
- Coder 自动检查已通过；Orchestrator 仅复测 RF1 深色主题和搜索栏边线，RF2/RF3 无需重跑。

## 后续产品决策

- 2026-08-13：Orchestrator 明确要求彻底删除 Tab 元信息/预览功能。该决定废止本 spec 当时已验收的 Tab 行为，但不改变 SPEC-12 在当时范围内的 `passed` 历史；删除工作纳入当前 SPEC-13 返工，并由 SPEC-14 做零引用清理回归。
- 安全不变量保持：`SearchResultItem` 继续不携带正文，不以“预览”名义扩大正文暴露。

## 完成定义

用户可观察到新版快速检索视觉，所有既有真实数据、双窗口和键盘行为保持通过；QA `passed`，状态回写完成。
