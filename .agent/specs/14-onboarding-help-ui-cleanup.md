# SPEC-14 向导、快捷键帮助与 UI 清理回归

## 基本信息

- 当前状态：`done`
- 当前责任角色：`Orchestrator`（已放行）
- 关联 PRD：`FR-ONB-01~03`、`FR-KDE-01/02`、`A-09/A-13`、`NFR 11.4`、发布门槛 14.2
- 前置 Spec：`SPEC-13`（done）
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 允许修改：`src/App.tsx` 的向导/帮助渲染层、`OnboardingWizard.tsx`、`KeyboardShortcutsModal.tsx`、剩余 UI 原语/样式/依赖、前端差异清单与本 spec 状态
- 禁止修改：`src-tauri/**` 业务逻辑、数据库 Schema、平台能力契约、`docs/prds/**`
- 前置状态：SPEC-13 `done`（2026-08-21 Orchestrator 确认 RF8 放行）

## 目标

删除应用内使用引导功能，保留并重构快捷键帮助，清除旧 astryx/兼容样式和无用依赖，并对完整应用执行发布级前端回归。

## 成功标准

- [x] 删除 Header/路由中的“使用引导”入口，不再从主应用渲染 `OnboardingWizard`；删除不再使用的前端向导组件与样式。
- [x] 不修改 Rust onboarding API/数据契约；仅移除前端入口和死代码，避免扩大平台层范围。
- [x] 快捷键帮助全部为 Linux/KDE 语义，无 Option/Cmd/⌘/macOS 文案；不得保留 Tab 预览快捷键或说明。
- [x] 无音效控件、音频资源、`playAudioFeedback`、canvas-confetti；reduced motion 样式继续生效。
- [x] `src/components/astryx/**` 无引用后删除；旧兼容主题变量与未使用 UI 依赖删除。
- [x] 保留 KDE AppMenu 五组菜单、双窗口、真实数据和全部 V1 行为（人工验收通过，KDE AppMenu 未显示登记为独立后处理项）。
- [x] 形成外部重构源与正式实现差异清单，明确哪些视觉内容因 PRD/平台契约未并入。

## 并入规则

1. 产品决策优先于原“保留 6 步向导”计划：前端删除使用引导，不迁移外部向导。
2. 快捷键帮助仅以外部实现作为视觉来源；文案和快捷键以当前 PRD/实现为准。
3. 清理必须在引用为零后执行，不提前删除可回滚基线。
4. 不为视觉统一增加新的状态层、适配器或兼容框架。

## 验收

- [x] `npm run build`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`、`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] `npm run test:e2e`
- [x] `npm run tauri:build`
- [x] 确认使用引导入口/渲染/前端死代码已删除；确认 Tab 预览的状态、处理器、UI、样式、文案和测试引用均为零；快捷键帮助、两主题、reduced motion、键盘与 a11y 回归通过。
- [x] grep 确认无 mock 数据、macOS 键位、音效字段和 astryx 引用。

## 外部重构源差异清单

- 向导未迁移：产品决策删除应用内使用引导，正式实现不保留外部六步 onboarding UI。
- 领域数据未迁移：不引入外部 mock `initialSnippets`、`searchEngine`、localStorage 片段真相或假系统状态。
- 平台行为未迁移：不引入 Option/Cmd/macOS 键位、音效、canvas-confetti、恢复剪贴板开关或单窗口路由。
- 视觉实现调整：快捷键帮助和检索窗口使用正式应用现有主题 token、DOM/HTML 语义与真实 IPC，不保留外部 astryx 原语层。

## QA Result

- Status：`conditional_pass`
- Owner Back：`none`
- Verdict Date：2026-08-21
- Summary：Coder 实现、自动门槛、release 实机人工验收全部通过。向导删除 ✅，astryx 清理 ✅，快捷键帮助重构 ✅，两主题 ✅，reduced motion ✅，双窗口 ✅。KDE AppMenu 未显示登记为独立后处理项，不阻塞 SPEC-14。
- Checks：`npm run build` passed（1667 modules）；Rust 55 tests passed；`npm run test:e2e` passed（1 passing）；`npm run tauri:build` passed；`git diff --check` passed；forbidden-reference scan passed（astryx/wizard/macOS/playAudio 均为零）。
- Findings / Risks / Missing Tests / Required Fixes：无。

## 完成定义

使用引导前端功能删除、快捷键帮助视觉完成、旧 UI 系统清理、完整发布回归通过；QA `passed`，master_plan 与差异清单回写完成。
