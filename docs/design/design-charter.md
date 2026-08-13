# Searchis 设计宪章

> 本文件是项目的设计 PRD。修改它 = 需求变更，必须走 Change 流程（Design Impact = design_system）。
> 每个视觉 spec 的第一句必须引用本宪章。
> 状态：v0.1 实践版（随 SPEC-15 practice 落地）；参照视觉：~/hermes/design/searchis/searchis-design-directions.html 的 D 变体。

## Design Read（一句话）

Searchis 是键盘优先的本地片段检索工具（rofi/krunner/Raycast 一族）：液态玻璃质感、深色优先、
单一强调色、等宽字体承载代码气质、克制动效、信息密度 4（快速窗口）~ 5（管理窗口）。

## 三旋钮

- DESIGN_VARIANCE：5（对称工具布局，允许轻微不对称留白）
- MOTION_INTENSITY：4（窗口入场 + 选中态 + 主题切换，其余静态）
- VISUAL_DENSITY：4-5（快速窗口每行 ≤ 4 信息元素）

## 设计原则

1. 键盘优先：界面为"输入 → 选中 → 粘贴"服务；任何不加速该闭环的装饰都是噪音。
2. 双窗口同一产品：快速检索窗口与管理窗口共用同一 token 体系，禁止各自为政。
3. 玻璃是签名：快速检索窗口为全玻璃面板（blur 40px + 内发光描边 + 高光叠加）；
   管理窗口选择性玻璃（Header/侧栏），正文区实色保证对比度。
4. 动效有动机：只保留 popIn 入场、选中态过渡、主题切换三处动效。
5. 隐私即设计：搜索窗口永不展示正文；敏感片段默认遮挡，遮挡是视觉语言的一部分。

## 签名元素

- Key 徽标（mono 首字符方块，选中态高亮）
- Key 高亮 mark（accent 色 mono 加粗，无背景）
- 敏感遮挡（锁图标 + "敏感 · 已隐蔽" + 圆点遮罩）

## 令牌要点（完整表见 design-tokens.md）

- 圆角：窗口 22px / 控件 12px / 徽标 8px / 胶囊 9999px（唯一规则）
- 强调色：默认电光蓝 #57a7ff（深）/ #2f6df6（浅）；accent 色相必须明暗一致
- 已知不一致（待修）：当前 --quick-accent 浅色 hue 35（暖）vs 深色 hue 200（蓝）
- 字体：UI 待定（Figtree 或系统栈，禁止声明未打包字体）；Mono = JetBrains Mono（已打包）
- 玻璃透明度上限：深 0.85 / 浅 0.75（亮壁纸对比度红线）

## 反模式（禁止清单）

- 霓虹光晕、AI 紫渐变、三列等宽卡片
- 快速窗口每行超过 4 个信息元素
- 搜索窗口出现正文预览（隐私不变量）
- localStorage 作为主题/设置第二真相源
- 页面中途明暗主题翻转
- 组件硬编码颜色/圆角（必须引用 token）

## 维护

- 版本：v0.1（practice）
- 最后更新：2026-08-13
- 相关文件：docs/design/design-tokens.md / docs/design/visual-qa-checklist.md
