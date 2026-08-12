# SPEC-14 向导、快捷键帮助与 UI 清理回归

## 基本信息

- 当前状态：`todo`
- 当前责任角色：`Coder`
- 关联 PRD：`FR-ONB-01~03`、`FR-KDE-01/02`、`A-09/A-13`、`NFR 11.4`、发布门槛 14.2
- 前置 Spec：`SPEC-13`（done）
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 允许修改：`src/App.tsx` 的向导/帮助渲染层、`OnboardingWizard.tsx`、`KeyboardShortcutsModal.tsx`、剩余 UI 原语/样式/依赖、前端差异清单与本 spec 状态
- 禁止修改：`src-tauri/**` 业务逻辑、数据库 Schema、平台能力契约、`docs/prds/**`

## 目标

完成重构版 OnboardingWizard 与 KeyboardShortcutsModal 的视觉并入，清除旧 astryx/兼容样式和无用依赖，并对完整应用执行发布级前端回归。

## 成功标准

- [ ] 保留 6 步向导，步骤进度、跳过/完成和异常续接仍由真实 onboarding API 驱动。
- [ ] 环境检测显示真实 Arch/KDE/X11/KGlobalAccel/剪贴板/X11 注入能力，不复制外部假“已授权”状态。
- [ ] 快捷键帮助全部为 Linux/KDE 语义，无 Option/Cmd/⌘/macOS 文案。
- [ ] 无音效控件、音频资源、`playAudioFeedback`、canvas-confetti；reduced motion 完整生效。
- [ ] `src/components/astryx/**` 无引用后删除；旧兼容主题变量与未使用 UI 依赖删除。
- [ ] 保留 KDE AppMenu 五组菜单、双窗口、真实数据和全部 V1 行为。
- [ ] 形成外部重构源与正式实现差异清单，明确哪些视觉内容因 PRD/平台契约未并入。

## 并入规则

1. 外部向导/帮助仅作为视觉来源；业务状态、文案、快捷键和平台检测以当前 PRD/实现为准。
2. 清理必须在引用为零后执行，不提前删除可回滚基线。
3. 不为视觉统一增加新的状态层、适配器或兼容框架。

## 验收

- [ ] `npm run build`
- [ ] `cargo test`、`cargo clippy --all-targets -- -D warnings`
- [ ] `npm run test:e2e`
- [ ] `npm run tauri:build`
- [ ] 6 步向导实机、快捷键帮助、KDE AppMenu、两主题、reduced motion、键盘与 a11y 回归。
- [ ] grep 确认无 mock 数据、macOS 键位、音效字段和 astryx 引用。

## QA Result

- Status：`not_run`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

向导/帮助视觉完成、旧 UI 系统清理、完整发布回归通过；QA `passed`，master_plan 与差异清单回写完成。
