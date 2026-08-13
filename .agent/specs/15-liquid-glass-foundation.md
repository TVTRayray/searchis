# SPEC-15 液态玻璃地基（practice 演示版）

> 本 spec 是"设计维度纳入工作流"的实践演示（branch: design-practice）。
> 演示点：Design Impact 申报、设计工件引用、视觉证据前置、QA 视觉验收。
> 正式版应由 Master 在 .agent/ 流程中立项；本文件格式对齐 spec.template.md。

## 基本信息

- Spec ID：15（practice）
- 标题：液态玻璃地基：快速检索窗口玻璃化 + 设计工件落地
- 当前状态：`in_progress`（Coder = Hermes practice 会话）
- 关联阶段：Phase 7 后续（视觉重构第 3 轮）
- 当前责任角色：`Coder`

## Design Impact

- 档位：`design_system`（新增 docs/design/ 三件套 + 玻璃 token 首落地）
- 影响面：快速检索窗口（.quick-search-shell / .quick-search-window）、tauri.conf.json（search 窗口透明）
- 证据路径：docs/design/design-charter.md、docs/design/design-tokens.md（参照 searchis-design-directions.html D 变体）

## 背景与目标

searchis 视觉重构（SPEC-12/13/14）存在三套样式系统并存、主题双真相源、
快速窗口无圆角（QA finding）等问题。本 spec 落地设计地基的第一层：
把液态玻璃配方应用到签名界面（快速检索窗口），并把设计工件（宪章/tokens/验收清单）
纳入仓库 docs/design/，为后续 SPEC-16/17/18 提供参照。

## 用户故事 / 成功标准

- 作为：键盘优先用户
- 我希望：Alt+O 呼出的检索窗口是真正的玻璃面板（圆角、内发光、高模糊）
- 从而：窗口与桌面融合，视觉质感接近 macOS Liquid Glass

成功标准：

- [x] .quick-search-window 应用玻璃配方：blur 40px + 内发光描边 + 高光叠加 + 圆角 22px
- [x] search 窗口启用透明（tauri.conf.json transparent: true）
- [x] docs/design/ 三件套（宪章/tokens/视觉验收清单）入库
- [x] npm run build 通过
- [ ] 实机（Arch/KDE/X11）视觉验收：亮壁纸对比度、黑闪、圆角（QA 待办）

## 非目标

- 管理窗口玻璃化（SPEC-17）
- accent 色相统一（SPEC-16，已记入宪章"已知不一致"）
- 主题单一真相源（localStorage 移除，SPEC-15 正式版）
- legacy token 清除（SPEC-15 正式版，避免与在途 SPEC-13 工作冲突）

## 接口与契约

- 输入：无新 IPC
- 输出：无契约变化
- IPC / API：无
- 异常 / 错误返回：无

## 数据与状态变化

- 新增状态：无
- 变更状态：无
- 持久化影响：无

## 边界与失败场景

- 透明窗口在无合成器环境（X11 无 KWin）下：玻璃降级为不透明实色（backdrop-filter 失效时
  background 兜底为 --quick-surface 高不透明度值），功能不受影响
- 与管理窗口在途 SPEC-13 工作冲突：本 practice 在独立 worktree + 独立分支执行，不触碰
  ManagerWindow.tsx / App.tsx / api/snippets.ts / HeaderBar / SettingsModal

## 实施要求

- 只允许修改：src/index.css（.quick-search-shell / .quick-search-window 段）、
  src-tauri/tauri.conf.json（search 窗口 transparent）、新增 docs/design/**
- 禁止：组件 .tsx、Rust、IPC、存储契约

## 测试点

- [x] npm run build（tsc + vite build）
- [ ] 实机：Alt+O 呼出、Esc 关闭、失焦关闭、明暗切换、亮壁纸对比度（QA）

## QA Result

- Status：`not_run`（practice 交付后由用户/QA 执行）
- Owner Back：`QA`
- Required Fixes：待定
- Retest Criteria：实机视觉验收清单（docs/design/visual-qa-checklist.md 玻璃与透明窗口节）

## 完成定义

- [x] 快速窗口玻璃配方落地
- [x] 透明窗口配置
- [x] 设计工件入库
- [x] build 通过
- [ ] QA 实机视觉验收
