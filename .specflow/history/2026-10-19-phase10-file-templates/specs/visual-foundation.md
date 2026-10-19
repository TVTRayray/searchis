# Delta Spec: visual-foundation

> Change: `phase10-file-templates`
> Status: `draft`
> 原 SPEC-20。详细契约（允许/禁止修改范围、token 清单、边界场景、测试点）：[`contracts/20-botanical-ember-visual-foundation.md`](../contracts/20-botanical-ember-visual-foundation.md)，实现与 QA 必读。
> PRD：A-17、FR-VIS-01、FR-VIS-02、FR-SET-01（主题展示名）、NFR 11.4、AC-38、D-17。设计源：`docs/design/`、`docs/prds/file-templates-ui-theme-preview.html`。

## ADDED Requirements

### Requirement: 不透明主题，无玻璃效果

所有窗口与表面必须（MUST）不透明；源码中不得存在 `backdrop-filter`、`--color-surface-glass-*`、`--color-canvas-glow-*`、`--blur-radius` 及 `.main-window-shell::before` 氛围光。`main` 窗口 `transparent` 必须为 `false`；`search` 窗口保持 `transparent: true` 仅用于显示 26px 圆角。

#### Scenario: 机械检查无玻璃残留

- **GIVEN** 本 spec 实现完成
- **WHEN** 执行 `grep -rn "backdrop-filter\|glass\|canvas-glow\|Figtree\|HeaderBar" src/`
- **THEN** 无任何结果（含注释）

### Requirement: 植物 / 暖炭双主题 token

`src/index.css` 的浅色（`:root`）与深色 token 取值必须（MUST）与 `docs/design/design-tokens.md` 颜色表逐项一致，并声明 `--color-highlight`、文件类型色、语法高亮色、形状、字体与 `--grain-opacity` token。组件不得新增硬编码颜色。

#### Scenario: 切换主题仅改变颜色

- **GIVEN** 管理窗口已打开
- **WHEN** 用户在“浅色 植物”“深色 暖炭”“跟随系统”之间切换
- **THEN** 颜色按 token 表变化，圆角、字体、布局不变，且无白闪

#### Scenario: 首帧主题正确

- **GIVEN** 上次主题为“深色 暖炭”
- **WHEN** 应用重启
- **THEN** 首帧即按深色渲染，不闪烁到浅色

### Requirement: 共享形状、字体与纹理

两套主题必须（MUST）共用：窗口 26px、卡片与代码块 18px、列表行 14px、输入框/按钮/快捷键标签/徽标胶囊形；标题字体 Playfair Display（仅 ≥18px 标题，中文不加斜体），正文 Source Sans 3，等宽 JetBrains Mono；两种拉丁字体的 woff2 子集随包发货，Figtree 全部删除；窗口背景有 `pointer-events:none` 的 SVG 纸张颗粒层。

#### Scenario: 字体真实加载

- **GIVEN** 目标机未额外安装 Playfair Display、Source Sans 3
- **WHEN** 打开管理窗口详情区
- **THEN** 标题与正文使用随包字体渲染，中文回退到系统 Noto CJK

### Requirement: 控件与动效规范

输入框必须（MUST）为填充底、无边框、焦点时 2px 强调色光环带偏移；主按钮实心、次要按钮描边、每个界面最多一个主按钮；lucide 图标 `strokeWidth={1.5}`。快搜窗口选中/悬停过渡 ≤150ms、弹出 ≤200ms；管理窗口悬停 ≤300ms ease-out；列表不做卡片上浮；`prefers-reduced-motion: reduce` 时关闭非必要动画。

#### Scenario: 减少动态效果

- **GIVEN** 系统开启 `prefers-reduced-motion: reduce`
- **WHEN** 用户在快搜窗口上下移动选中项
- **THEN** 选中态即时切换，无过渡动画

### Requirement: HeaderBar 按钮迁移到左栏底部

`HeaderBar.tsx` 必须（MUST）删除且零引用；快捷键速查、主题切换、设置三个按钮横向排列于管理窗口左栏底部，行为与原 HeaderBar 一致；左栏标题旁的重复设置按钮删除。

#### Scenario: 左栏底部按钮可用

- **GIVEN** 管理窗口已打开
- **WHEN** 用户依次点击左栏底部三个按钮
- **THEN** 分别打开快捷键速查、切换主题、打开设置；KDE Global Menu 五组菜单仍显示

### Requirement: 主题展示名

设置中的主题名称必须（MUST）显示为“浅色 植物”“深色 暖炭”“跟随系统”；存储值仍为 `light`/`dark`/`system`，不迁移数据。

#### Scenario: 旧设置保持有效

- **GIVEN** 升级前设置 `theme=dark`
- **WHEN** 升级后打开设置
- **THEN** 选中项显示“深色 暖炭”，存储值仍为 `dark`

### Requirement: 主题对比度达标

浅色与深色下正文、muted 文字、选中态文字、主按钮文字的对比度必须（MUST）满足 WCAG AA（正文 ≥ 4.5:1），复核数值写入 result 证据。

#### Scenario: 对比度复核

- **GIVEN** 两套主题 token 已落地
- **WHEN** 用工具测量上述四类文字与其背景
- **THEN** 全部 ≥ 4.5:1，数值记录在实现 result 中

## Non-goals and Risks

- 非目标：分组树、类型筛选、语言高亮、文件模板 UI、设置视图 S1、快捷键速查内容改版（后续 spec）；不改任何键盘、检索、复制、粘贴、设置持久化行为。
- 风险：KWin 合成器关闭时快搜窗口四角为直角黑底（已接受）；颗粒不透明度需实机调定后写回 `docs/design/design-tokens.md`。
