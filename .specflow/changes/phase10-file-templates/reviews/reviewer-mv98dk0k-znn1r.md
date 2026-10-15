# QA Report: reviewer-mv98dk0k-znn1r

## Verdict

`repair` — category: `accessibility`。审查已完成，不等于 QA 通过；不修改业务代码、不推进 state。

范围：仅 visual-foundation / SPEC-20。阅读任务包、proposal、tasks、Delta Spec、contract 20、design、ux-design、设计宪章/tokens/视觉清单，核对工作区真实 diff 和两轮 implementer result。最新 completed 为 implementer-mv98bmof-n2bpf；前轮实现证据见 implementer-mv97q3xa-qsqnu。工作区另有 Planner/runtime 文件变更，未把它们视为本 reviewer 的修改。

## Spec Coverage

| Requirement / scenario | Evidence | Result |
|---|---|---|
| 不透明主题、无玻璃残留 | src 机械 grep 零结果；tauri.conf.json main=false/search=true；E2E 配置继承窗口配置 | 静态通过；实际 KWin 圆角 pending_human |
| 双主题 token、切换仅改颜色 | index.css 对照 design-tokens；14 项具完整 token 名的明暗值脚本比较通过，其余组合/语义命名表行人工源码对照；已采用人工决策 #B35838 | 静态通过；切换运行证据仅来自 implementer E2E |
| 首帧正确、无白闪 | main.tsx 挂载前缓存恢复逻辑未改；ThemeProvider 仍从后端纠正；main 现在不透明 | 源码路径保留；首帧实机 pending_human，未声称实际验证 |
| 字体、形状、纹理 | 六个 woff2 被本轮 build 输出；OFL 文件在仓库；SVG ::after pointer-events:none；共用 radius/font tokens | 源码/打包通过，视觉对照 pending_human |
| 控件、reduced-motion | 全局焦点样式存在，但快搜覆盖删除；主题过渡覆盖 reduced-motion | **失败 F1、F2** |
| HeaderBar 迁移 | App/ManagerWindow diff：移除 HeaderBar，底栏绑定原 shortcuts/theme/settings 回调，重复设置按钮删除；src 零引用 | 静态通过；Global Menu pending_human |
| 主题展示名/旧值有效 | ThemeProvider labels 与 SettingsModal；light/dark/system 存储协议未改 | 静态通过；implementer 有后端持久化 E2E 证据 |
| 对比度 | 深色主按钮本轮复算 4.5680；实现记录其他正文/选中组合 ≥4.5；placeholder 实际使用 muted/field | **失败 F3** |

## Findings

### F1 — 快搜输入框明确取消焦点环，测试还锁定旧行为

- Severity: medium
- Category: `accessibility`
- Expected: Delta「控件与动效规范」与 contract 要求输入框 focus 有 2px 强调色光环及偏移。
- Actual: `src/index.css:1154-1158` 对快搜 input 的 focus/focus-visible 设置 `outline:none !important; box-shadow:none`，覆盖 `:focus-visible` 的 2px outline。不是单纯去除边框。
- Reproduction / evidence: 打开快搜、聚焦 `input[cmdk-input]`；按 CSS cascade 不会出现规范焦点环。`e2e/create-and-search.e2e.ts:389-402` 反而要求 outlineStyle 与 boxShadow 都为 none，因此现有 E2E 通过不能证明该要求成立。
- Required repair: 保留输入框无边框，同时为快搜提供符合规范的焦点环（可在适当容器实现）；把这条旧 E2E 断言替换为实际检查焦点指示器宽度、颜色、偏移的正向断言。

### F2 — reduced-motion 被更高 specificity 的 important 主题过渡覆盖

- Severity: medium
- Category: `accessibility`
- Expected: 开启 reduced-motion 时非必要过渡关闭，状态即时切换。
- Actual: `src/index.css:177-184` 的 `.theme-transition *` 使用 `transition: ... var(--motion-manager) ... !important`（220ms）；`:326` 起 reduce 媒体规则只在 `*` 上设置 `transition-duration:0.01ms !important`。同为 important，前者 specificity 更高，shorthand 的 duration 胜出。`ThemeProvider.tsx:72-78` 每次主题更新都会添加该 class，持续约 300ms。
- Reproduction / evidence: reduced-motion=reduce，切换明暗主题，在 theme-transition class 尚在时检查普通按钮/列表行的 computed transition-duration，会受 220ms 主题规则控制。这是源码 cascade 核对，非本轮运行时探针。
- Required repair: 在 reduce 条件下有效覆盖主题过渡或不启用它；补充覆盖 theme-transition 存在期间的 reduced-motion 回归断言，不只测类移除后。

### F3 — 浅色 muted placeholder 在实际输入框底上只有 4.24:1

- Severity: medium
- Category: `accessibility`
- Expected: Delta「主题对比度达标」要求 muted 文字 ≥4.5:1，未规定 placeholder 例外；WCAG 文字对比度也不因它是 placeholder 自动豁免。
- Actual: `src/index.css:481,736,1051` placeholder 使用 `--color-fg-muted`；填充背景 `--color-field-bg` 为 #EDE8DC，浅色文字 #66705F，本轮 Node WCAG 相对亮度复算 **4.2435:1**。implementer 已记录这个问题，但以“不在 MUST 列表内”为由保留不成立。
- Reproduction / evidence: 浅色下查看管理过滤框、编辑输入框/textarea、快搜输入框的提示文字；实现报告中的 4.24 与本轮计算一致。
- Required repair: 不更改已批准 token 表，改用现有能达标的文字 token（例如 secondary）或等价合规配对；复算实际背景并增加回归检查。

## Tests Reviewed

本轮实际执行：

- `npm run build`：**pass**，tsc + Vite，1667 modules；六个新增字体都进入产物。
- `git diff --check`：**pass**。
- `grep -rnE 'backdrop-filter|glass|canvas-glow|Figtree|HeaderBar' src/`：零结果（预期 exit 1）。
- `grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components src/App.tsx`：零结果。
- Node token 对照：**pass**，14 个完整命名明暗颜色比较；RGBA 小数表示规范化后无差异。第一次文本比较把 .55/0.55 当成差异，已校正重跑；不是产品缺陷。组合表行另作源码对照。
- Node WCAG 复算：深色主按钮 #FFF8F2/#B35838 = **4.5680:1**；浅色 placeholder #66705F/#EDE8DC = **4.2435:1（不合格）**。
- `git diff --name-only -- src-tauri/src src/api src/types src/lib`：零结果。
- Rust scoped test：not_applicable；此 slice 无 Rust 逻辑变更。
- `npm run test:e2e`：**本轮未执行**。测试源码含 `xdotool windowactivate --sync`（如 :370、406、434）及后续桌面按键路径；未获允许真实前台输入，不重跑整个套件。实现者报告曾通过（最新 1 passing，7.6s），属于转引证据，不是本轮通过；且 F1 的断言错误、F2/F3 缺少有效覆盖。

## Design Conformance

逐条对应 docs/design/visual-qa-checklist.md；`[ ]` 标明失败、未验或后续 slice，不代表全部为 blocker。

### 一致性
- [x] 所有表面引用同一 token；组件/App 硬编码颜色 grep 零结果，组件圆角使用 CSS token 类，未见新增数字圆角类。
- [x] 强调色、圆角、字体在两主题共用样式；源码符合大体方向。

### 主题与对比度
- [ ] 明暗关键面截图：pending_human，本轮不自动拍摄实机。
- [ ] 对比度：主按钮决策修复有效；muted placeholder 不达标（F3）。
- [ ] 切换即时、无白闪及重启首帧：实现者有主题切换 E2E 证据；首帧与无白闪 pending_human。

### 字体与圆角
- [x] 字体声明与随包资源对应，build 实际包含；Figtree 零引用；运行加载证据转引实现者 E2E。
- [x] 主体圆角档位源码采用 window/card/row/pill tokens；KWin 物理窗口圆角 pending_human。
- [x] 未见中文斜体；衬线仅在 ≥18px 标题使用。

### 动效与无障碍
- [x] 常规交互反馈为 quick 120ms、pop 180ms、manager 220ms；无列表上浮。
- [ ] reduced-motion：F2。
- [ ] 焦点与键盘：F1；完整键盘运行回归本轮未执行。

### 状态完整性
- [x] 本 slice 保留现有 loading/empty/error 结构；扫描/导入/分组/文件筛选新增状态不在 SPEC-20。
- [ ] 文件缺失/自动导入/导出路径：not_applicable，后续 slice。
- [x] 三栏/快搜原信息密度基本保留，无本 slice 外新 UI 结构。

### 隐私与安全（视觉侧）
- [x] 敏感遮挡与编辑正文挂载条件未改；本轮仅静态回归，未宣称动态安全测试通过。
- [ ] 文件模板敏感开关/内容预览：not_applicable，后续 slice。
- [x] 未新增业务日志/内容提示路径；API/types/lib/Rust 无 diff。

### 回归
- [ ] 完整键盘及 IME 动态回归未执行；实现者 E2E 通过不能替代所有键盘场景。
- [x] 设置仍经后端协议持久化；localStorage 仅沿用主题缓存，未变更行为。

### Searchis 专项
- [x] backdrop-filter、玻璃 tokens、氛围光、HeaderBar 源码零引用。
- [x] token 表源码对照，含已批准深色主按钮修正。
- [x] 本 slice 删除 HeaderBar 且无引用；改动中的组件仍有 import 使用（AboutModal 保留用于现有设置，删除属后续 slice）。未做仓库全量死代码认证。
- [x] 纸张层为 SVG 且 pointer-events:none；颗粒视觉强度 pending_human。
- [ ] Q-A 类型/分组信息与徽标新规则：not_applicable，后续 slice。
- [x] 本 slice 主/次操作样式调整符合每界面一个主操作的源码结构；运行探针证据转引实现者。
- [ ] KWin 圆角、KDE Global Menu 五组菜单：pending_human。

## 实机验收

以下不阻止单独的自动化 QA 结论，但必须在归档前由人记录到 test_report.md；本次 repair 原因是 F1–F3，不是这些待验项：

- pending_human：浅/深各六张截图（快搜、管理、设置、快捷键、关于、确认），对照定版预览。
- pending_human：底栏按钮实际可用与 KDE Global Menu 五组菜单。
- pending_human：KWin 26px 圆角、合成器关闭已知降级、颗粒最终调定。
- pending_human：不透明主窗口冷启动深色首帧与主题切换无白闪。
- pending_human：Noto CJK 缺失回退及全键盘/IME 实机回归。

## Next Step

`repair`：仅把 F1–F3 退回 Implementer；沿用批准的 token 表，不扩张业务行为。修复后复核焦点环、reduced-motion 的主题更新期间、实际 placeholder 背景配对；全桌面 E2E 如需执行，应先取得明确授权或由 orchestrator 提供安全隔离运行方式。由 runtime 导入结果并决定后续状态，本 reviewer 不自行标记 QA passed。
