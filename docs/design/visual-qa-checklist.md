# Searchis 视觉验收清单（Visual QA Checklist）

> 由设计宪章推导。QA 对 Design Impact ≠ none 的 change 必须逐条执行，
> 结果写入 QA Report 的 Design Conformance 节；不合格退回 Coder 并附截图证据。

## 一致性

- [ ] 快速检索窗口与管理窗口使用同一 token 体系（grep 组件无硬编码颜色/圆角）
- [ ] 强调色、圆角、字体全应用一致
- [ ] accent 明暗双模色相一致（当前已知违反：--quick-accent 浅 hue 35 / 深 hue 200）

## 玻璃与透明窗口

- [ ] 快速窗口透明生效：KWin 下无黑闪，圆角 22px 与 CSS 一致
- [ ] 窗口拖拽/失焦关闭行为与 Gate 0 基线一致
- [ ] 管理窗口为选择性玻璃（Header/侧栏），正文区实色
- [ ] 主窗口保留 KDE 原生标题栏（decorations 未禁用，无 data-tauri-drag-region 自绘区）
- [ ] prefers-reduced-transparency 下玻璃降级实色，布局无位移

## 亮壁纸对比度（实机必测）

- [ ] 纯白壁纸下正文对比度 ≥ 4.5:1（fg / fg-2）
- [ ] 高饱和壁纸（红/橙/青）下结果行可读，选中态可辨
- [ ] 玻璃透明度未突破红线（深 0.85 / 浅 0.75）

## 主题与对比度

- [ ] 明/暗双模关键面截图对比，无反色/低对比错误
- [ ] 主题切换即时生效，无白闪/跳变（RF1 回归）

## 字体与圆角

- [ ] 声明字体真实加载（Figtree 决策后必查，当前为系统回退）
- [ ] 圆角规则四档全应用，无规则外圆角

## 动效与无障碍

- [ ] popIn 生效；prefers-reduced-motion 下瞬时
- [ ] 焦点可见（focus-visible），键盘全程可操作
- [ ] 快速窗口呼出 ≤ 200ms（目标机实测）

## 状态完整性

- [ ] loading / empty / error 状态齐全（检索无结果、IPC 失败、敏感项）
- [ ] 快速窗口每行信息元素 ≤ 4

## 隐私与安全（视觉侧）

- [ ] 搜索窗口任何状态不出现片段正文
- [ ] 敏感片段显示遮挡与锁标，管理窗口 detail 默认遮挡

## 回归

- [ ] 键盘语义无回归：Esc / Enter / Ctrl+Enter / Tab / Ctrl+N / 方向键
- [ ] 设置持久化走后端契约（非 localStorage 真相源，SPEC-15 后续任务）
