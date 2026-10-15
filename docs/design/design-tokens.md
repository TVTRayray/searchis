# Design Tokens: Searchis

> 令牌规格。原则：Token 即代码。本文件是决策记录；代码中的 token 定义（`src/index.css`）才是真相。
> 组件禁止硬编码颜色、圆角、间距；全部引用 token。
> 来源：`docs/prds/file-templates-ui-requirements.md` 第二部分第 6 节（2026-10-15 定版）。

## 色彩（明暗双模）

| Token | 浅色「植物」 | 深色「暖炭」 | 用途 |
|---|---|---|---|
| `--color-canvas` | `#F2EEE2` | `#1A1917` | 窗口底色 |
| `--color-surface` | `#FAF8F1` | `#262624` | 详情区 / 卡片 |
| 侧栏底 | `#EDE8DA` | `#1F1E1D` | 管理窗口左栏、设置导航 |
| 列表栏底 | `#F6F3EA` | `#232321` | 管理窗口中栏 |
| 输入框 / 快捷键标签底 | `#EDE8DC` | `#30302E` | 填充式输入、Kbd |
| 代码块 / 底栏 | `#F2EEE3` / `#F3EFE5` | `#1F1E1D` | CodeMirror 区、快搜底栏 |
| `--color-fg` / secondary / muted | `#2D3A31` / `#46524A` / `#66705F` | `#F5F4EE` / `#C2C0B6` / `#9C9A92` | 文字三级 |
| `--color-fg-disabled` | `#A8AE9F` | `#6B6A64` | 禁用文字 |
| `--color-border` / strong | `#E3DDD0` / `#CFC7B6` | `rgba(222,220,209,.11)` / `.22` | 边框 |
| `--color-accent`（文字、图标） | `#5E7056` | `#E5916F` | 强调色 |
| 主按钮底 / 文字 | `#2D3A31` / `#F9F8F4` | `#B95C3D` / `#FFF8F2` | 主按钮 |
| 主按钮悬停 | `#A65A44` | `#A9522F` | 主按钮 hover |
| `--color-btn-secondary-border` | `#8C9A84` | `rgba(229,145,111,.55)` | 次要按钮描边 |
| 选中背景 / 选中描边 | `rgba(140,154,132,.20)` / `.60` | `rgba(217,119,87,.16)` / `.45` | 列表选中 |
| `--color-icon-bg` | `rgba(140,154,132,.16)` | `rgba(217,119,87,.14)` | 类型图标圆形底 |
| `--color-focus-ring` | `#8C9A84` | `#D97757` | 焦点光环 |
| `--color-highlight` | `rgba(214,180,90,.34)` | `rgba(214,170,80,.30)` | 匹配高亮 |
| 阴影颜色 | `rgba(45,58,49,…)` 柔和扩散 | `rgba(0,0,0,.45)` | 窗口 / 浮层阴影 |
| success / warning / danger | `#4D7C3A` / `#A86A12` / `#B23A2E` | `#8FBF7A` / `#E0B05A` / `#E5806F` | 语义色 |
| `--color-file-doc/sheet/pdf/code` | `#5F7183` / `#5E7056` / `#B0664F` / `#4B5850` | `#93A9C6` / `#9BBF84` / `#E3907C` / `#B5AFA2` | 文件类型图标 |
| `--color-syntax-keyword/string/comment/number` | `#A65A44` / `#5E7056` / `#8A8F84`（斜体）/ `#5F7183` | `#E59A7A` / `#A9C98F` / `#8A877E`（斜体）/ `#9DB4D3` | 语法高亮 |

对比度估算（实现时用工具复核并回写）：浅色正文约 11:1、muted 约 4.8:1；深色 muted 约 5.4:1。

## 圆角

| 档位 | 值 | 应用 |
|---|---|---|
| `--radius-window` | 26px | 快搜窗口（需 `search` 窗口透明 + KWin 合成器） |
| `--radius-card` | 18px | 卡片、代码块、文件信息卡片 |
| `--radius-row` | 14px | 列表行 |
| `--radius-pill` | 999px | 输入框、按钮、快捷键标签、类型徽标、标签 |

## 字体

- 标题：`--font-heading` = `"Playfair Display", "Noto Serif CJK SC", serif`，500 / 600 / 500 italic；只用于 ≥18px 标题，中文不斜体。
- 正文：`--font-body` = `"Source Sans 3", "Noto Sans CJK SC", sans-serif`，400 / 500 / 600。
- 等宽：`--font-mono` = JetBrains Mono。
- 拉丁子集 woff2 随包发货（`src/assets/fonts/`，OFL）；中文回退系统 Noto CJK（`noto-fonts-cjk`）。Figtree 已废弃。
- 小标签与按钮文字字距 0.04–0.06em，不全大写。

## 阴影

- 窗口阴影：浅色 `rgba(45,58,49,…)` 柔和大扩散；深色 `rgba(0,0,0,.45)`。
- 悬浮阴影：仅用于弹出菜单与对话框；列表行不加阴影。
- 无内发光、无玻璃高光叠加。

## 间距与动效

- 纹理：`--grain-opacity` 浅色约 0.06、深色约 0.035（实机调定后在此回写最终值）。
- 图标：lucide `strokeWidth={1.5}`。
- 快搜窗口：选中 / 悬停过渡 ≤150ms，弹出 ≤200ms。
- 管理窗口：悬停 ≤300ms ease-out；主题切换无过渡闪烁。
- `prefers-reduced-motion: reduce`：关闭全部非必要过渡，状态即时切换。
