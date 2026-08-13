# Searchis 设计令牌（Design Tokens）

> Token 即代码：本文件是决策记录；src/index.css 中的变量定义是真相。
> 组件禁止硬编码颜色、圆角、间距。
> 状态：v0.1 practice。完整玻璃面板配方见 liquid-glass 规范（~/hermes/design/searchis/）。

## 色彩（目标状态，随 SPEC-16 生效）

| Token | 深色 | 浅色 | 现状 |
|---|---|---|---|
| canvas | #0d1b2a | #f2f5f9 | 现状为中性灰系（--color-canvas），SPEC-16 决策是否更换 |
| surface（玻璃主面） | rgba(22,30,44,0.78) | rgba(255,255,255,0.70) | --quick-surface 已接近（0.78 / 0.88） |
| fg / fg-2 / fg-3 | #f4f7fb / #c3cdda / #8b98a9 | #17202e / #4d5b70 / #8a96a8 | 现状 oklch 系，语义等价 |
| border | rgba(255,255,255,0.16) | rgba(255,255,255,0.45) | --quick-border 已接近 |
| accent | #57a7ff | #2f6df6 | 不一致：现 --quick-accent 浅=oklch(48% .14 35) 深=oklch(72% .170 200)，色相分裂待修 |
| danger | #f87171 | #dc2626 | --color-danger 已存在 |

## 圆角（唯一规则，SPEC-16 全量应用）

| 档位 | 值 | 应用 |
|---|---|---|
| 窗口 | 22px | 快速检索窗口面板（practice 已落地） |
| 控件 | 12px | 按钮/输入/结果行选中态 |
| 徽标 | 8px | Key 徽标/Badge/Kbd |
| 胶囊 | 9999px | 标签/敏感标 |

现状：--radius-md 0.625rem（10px）等六档并存，SPEC-15/16 收敛为四档。

## 字体

- UI：Figtree（未打包，需决策：打包发货或改用系统栈；当前声明无效 = bug）
- Mono：JetBrains Mono 400/500/600/700（已随包）

## 玻璃面板配方（.quick-search-window，practice 已落地）

- backdrop-filter: blur(40px) saturate(180%)
- 内发光：inset 0 1px 0 rgba(255,255,255,0.28)（顶部）+ inset 0 -1px 0 rgba(255,255,255,0.06)
- 高光叠加：::before 径向渐变（22% 0% 处 rgba(255,255,255,0.14)）
- 内层描边：::after inset 1px rgba(255,255,255,0.08)
- 透明度红线：深 ≤ 0.85 / 浅 ≤ 0.75（亮壁纸对比度，QA 必测）

## 动效

- 入场 popIn：150ms cubic-bezier(0.16, 1, 0.3, 1)（scale 0.97 + 6px + fade）
- hover/active：120ms ease；主题切换：300ms ease
- prefers-reduced-motion：全部瞬时（已实现）

## 维护

- 版本：v0.1（practice）；最后更新：2026-08-13
- 相关：design-charter.md / visual-qa-checklist.md / liquid-glass-implementation-spec.md
