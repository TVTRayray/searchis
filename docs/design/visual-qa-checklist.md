# Visual QA Checklist: Searchis

> 由设计宪章推导的机械验收清单。QA 对 Design Impact ≠ none 的 change 必须逐条执行，
> 结果写入 QA Report 的 Design Conformance 节（逐条 `[x]` / `[ ]` + 证据）。不合格退回 Implementer 并附截图证据。
> 标注“实机”的条目若无法在自动化环境执行，写 `pending_human`，由人在归档审批前完成（见 change 的 `test_report.md`）。

## 一致性

- [ ] 所有窗口/表面使用同一 token 体系（grep 组件内无硬编码颜色/圆角：`grep -rnE "#[0-9a-fA-F]{3,8}\b|rgba?\(" src/components src/App.tsx` 仅允许 token 定义处）
- [ ] 强调色、圆角、字体全应用一致

## 主题与对比度

- [ ] 明/暗双模关键面截图对比，无反色/低对比错误（实机或 E2E 截图）
- [ ] 正文对比度 ≥ 4.5:1（WCAG AA），大字号 ≥ 3:1；muted、选中态、主按钮文字同样复核
- [ ] 主题切换即时生效，无白闪/跳变；重启首帧主题正确

## 字体与圆角

- [ ] 声明字体真实加载（非系统回退）；Figtree 零引用
- [ ] 圆角规则全应用（窗口 26 / 卡片 18 / 行 14 / 胶囊 999），无规则外圆角
- [ ] 中文无斜体；衬线标题只出现在 ≥18px 标题

## 动效与无障碍

- [ ] 动效有动机（层级/叙事/反馈/状态转换四类之一）；快搜 ≤150ms/≤200ms，管理窗口 ≤300ms
- [ ] prefers-reduced-motion 下全部瞬时
- [ ] 焦点可见（focus-visible 2px 光环带偏移），键盘全程可操作

## 状态完整性

- [ ] loading（扫描中、导入中）/ empty（列表、分组、文件筛选）/ error（错误码 + 下一步）状态齐全且指引明确
- [ ] 文件缺失标记、自动导入标记、导出路径提示可见
- [ ] 信息密度符合宪章（VISUAL_DENSITY 6）

## 隐私与安全（视觉侧）

- [ ] 敏感正文默认遮挡有效；敏感未展示时编辑器不挂载正文
- [ ] 文件模板不显示敏感开关，不预览文件内容
- [ ] 日志/提示不含查询词、正文、模板文件名或路径（导出成功提示中的路径除外）

## 回归

- [ ] 键盘语义无回归（Enter/Ctrl+Enter/Shift+Enter/Ctrl+O/Ctrl+E/Ctrl+N/Esc、IME 组合态）
- [ ] 设置持久化走后端契约（非 localStorage 真相源）

## Searchis 专项

- [ ] `grep -rn "backdrop-filter" src/` 无结果；玻璃 token 与氛围光零引用
- [ ] `src/index.css` token 取值与 `docs/design/design-tokens.md` 逐项一致
- [ ] 未引用组件清单：每个 `src/components/**/*.tsx` 导出的组件至少被一处 import（防 HeaderBar 式死代码复发），已删除组件（HeaderBar、AboutModal 等）零引用
- [ ] 纸张颗粒层 `pointer-events:none`，不拦截点击与拖入
- [ ] 快搜结果行 Q-A：类型图标、标题、`Key · 分组 · 使用次数`、徽标、`Ctrl+N` 序号、匹配高亮
- [ ] 徽标规则：文件为大写扩展名，文本仅 `language≠plain` 时显示语言名
- [ ] 每个界面最多一个主按钮
- [ ] 实机：快搜窗口 26px 圆角在 KWin 合成器开启时正确显示；KDE Global Menu 五组菜单显示
