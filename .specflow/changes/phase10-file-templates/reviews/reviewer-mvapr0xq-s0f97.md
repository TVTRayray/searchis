# QA Report: reviewer-mvapr0xq-s0f97

## Verdict

`repair` — category: `design_conformance`。

审查已完成，但不能给 SPEC-23 自动化 QA 通过结论：置顶文件模板的结果行丢失扩展名类型图标（R1）。Rust、构建和现有隔离 E2E 均通过，不代表这个未覆盖分支符合设计。

## Scope / Inputs

- 任务：`reviewer-mvapr0xq-s0f97`；lease：`lease-mvapr0xq-kebqs`；spec：`formatted-copy-export`。
- 已读任务包、项目宪法、proposal、tasks、Delta Spec、design、ux-design、`contracts/23-formatted-copy-and-export.md`、设计宪章/token/视觉清单、xlsx 预研与两份 implementer result。
- 两份实现结果合并审查：`implementer-mvan4gsr-gdmci`（原范围阻塞）及 `implementer-mvapijln-esj55`（补齐管理窗口按钮）。人工已解决 FileTemplateDetail 路径授权；不再将其列为权限阻塞。
- 快搜文件模板展示属于本 spec；FileTemplateDetail 仅核对动作按钮及其接线/反馈。Settings 可选、只读 exportDir 按已接受的非阻断项处理，设置界面留 SPEC-24。
- 未修改应用实现、规划工件或状态；未创建 subagent；未写真实剪贴板、未自动粘贴、未操作 WPS/Dolphin 或真实前台窗口。

## Spec Coverage

证据目录：`reviews/evidence/reviewer-mvapr0xq-s0f97/`（下文文件名均相对该目录）。

| Requirement / scenario | Evidence | Result |
|---|---|---|
| 按类型的 Enter / Ctrl+Enter / Shift+Enter / Ctrl+O / Ctrl+E / Ctrl+数字；底栏与快捷键分组 | QuickSearchWindow 的动作分派、KeyboardShortcutsModal 分组；`e2e.log` SPEC-23 用例验证底栏、IME、Enter、导出、打开、autoPaste=false；文本键盘回归用例亦通过 | 自动化已覆盖的路径通过；真实自动粘贴 pending_human |
| 输入法组合态不执行动作 | `onKeyDown` 首行 isComposing 守卫；隔离 E2E 组合态断言 | pass |
| 文件模板结果行扩展名图标、大写徽标 | 未置顶行 E2E 通过；`static-probe.log` 从实际 JSX 提取并执行图标表达式，置顶 xlsx 返回 Star 而非 FileTypeIcon | **repair：R1** |
| autoPaste 后端请求值与设置取“与” | `service.rs`、`service/template_actions.rs`；Rust FakeWm 断言与隔离 E2E autoPaste=false | pass（不声称实机注入通过） |
| X11 所有权确认、TARGETS/TIMESTAMP/INCR/SelectionClear、无连接错误 | `platform/x11_clipboard.rs` 及私有 Xvfb 测试，`cargo-all.log` | pass；生产桌面剪贴板兼容性 pending_human |
| md HTML/plain，GFM、原始 HTML 转义、链接白名单、无效 UTF-8 提示 | `format_copy/markdown.rs`；`cargo-format-copy.log` / `cargo-all.log` | pass；WPS/纯文本编辑器粘贴 pending_human |
| xlsx 活动表、样式展开/合并/数字日期/列宽行高、缓存值、HTML/TSV、50,000 上限、损坏拒绝 | `format_copy/xlsx.rs`、`xlsx/tests.rs`；Rust 测试，隔离 E2E 损坏文件失败不计数 | pass（格式输出断言）；WPS 还原效果 pending_human |
| 其他文件 URI/gnome-copied-files/plain | `format_copy/uri.rs` 测试及 `cargo-all.log` | pass；Dolphin 粘贴 pending_human |
| 成功持有后计数、operationId 幂等、失败不计数且保留窗口 | `service/template_actions.rs` 测试；隔离 E2E 成功计数、损坏文件/缺失文件失败路径 | pass |
| 导出命名、重名不覆盖、临时文件清理、默认目录回退、不存在目录不创建、幂等、窗口保持、路径反馈 | `templates/export.rs`、服务测试；隔离 E2E 临时目录导出与管理按钮 | pass（已测试路径） |
| 管理窗口按钮启用并复用后端命令 | ManagerWindow 调用 export/formatCopy，UUID operationId；formatCopy 固定 autoPaste=false；FileTemplateDetail 缺失时禁用、内联反馈；E2E 验证格式集合、计数与新增导出文件 | pass；内联反馈而非 toast 不构成违约 |
| NFR-PERF-06：各 100 次 | `perf.log`：release md 1 MB p95=24.1 ms；xlsx 5,000 单元格 p95=15.2 ms | pass |

## Findings

### R1 — 置顶文件模板以星标取代文件类型图标

- Severity: medium。
- Category: `design_conformance`。
- Expected: Delta Requirement「快搜结果行显示文件模板」、contract 成功标准及 UI requirements Q-A 要求文件模板始终按扩展名显示类型图标（淡色圆底）；置顶不能取代类型识别。
- Actual: `src/components/QuickSearchWindow.tsx:587–593` 的条件顺序是 `item.pinned ? Star : item.kind === "file" ? FileTypeIcon : languageIcon`。xlsx 一旦置顶，图标变成 Star；XLSX 徽标仍存在，不能替代明确要求的类型图标。
- Reproduction / evidence:
  1. 导入 xlsx 模板，在管理窗口启用“固定”，再在快搜按 Key 检索。
  2. 非固定行显示表格图标；固定后该位置只显示实心星标。
  3. 可复现静态渲染分支探针：`node .specflow/changes/phase10-file-templates/reviews/evidence/reviewer-mvapr0xq-s0f97/static-probe.mjs`。探针从当前源码 AST 提取 JSX 表达式执行；`static-probe.log` 记录 pinned=false → FileTypeIcon，pinned=true → Star、contractSatisfied=false。
  4. 这是实际分支的确定性代码证据，不是桌面截图或完整 React/DOM 验收；本轮未拍缺陷截图，渲染截图对照 pending_human。现有 SPEC-23 E2E 未覆盖置顶文件行，5 passing 不反驳该缺陷。
- Required repair: 文件模板分支优先并保留按扩展名的 FileTypeIcon；置顶状态可由既有分区/样式表达，不新增不必要 UI。补覆盖置顶 xlsx（及普通文件对照）的回归断言，断言实际类型图标，不能只检查 data-kind/data-category/徽标。只修此项并复跑相关检查。

## Tests Reviewed / Re-run

全部命令由 Reviewer 实际执行；完整输出保存在证据目录。

| Command | Result | Log |
|---|---|---|
| `cd src-tauri && cargo test format_copy`（先运行最小相关测试） | 21 passed，0 failed | cargo-format-copy.log |
| `npm run build` | pass；存在 >500 kB chunk 提醒，非构建失败 | build.log |
| `.specflow/bin/e2e-isolated.sh` | 5 passing；Xvfb + 私有 D-Bus + KWin；剪贴板/打开替身及临时 XDG/导出目录 | e2e.log |
| `cd src-tauri && cargo test` | 191 passed，0 failed | cargo-all.log |
| `cd src-tauri && cargo test --release qa_perf -- --nocapture` | 4 passed；含 NFR-PERF-06 两项与既有扫描性能项 | perf.log |
| `node …/static-probe.mjs` | 探针运行成功；复现 R1，不是应用符合性 pass；完成对比度与静态 import 审计 | static-probe.log |

隔离 E2E 的桌面会话输出含 systemd1 激活失败、xdg-desktop-portal-kde 退出噪声；测试进程退出成功、5 项通过，未据此宣称生产 KDE/portal 已验收。X11 Rust 测试使用其自身私有 Xvfb，不触及真实桌面选区。性能值为本次环境实测，不沿用 implementer 数字。

## Design Conformance

依据 `docs/design/design-charter.md`：纸面而非玻璃、键盘优先、植物/暖炭双主题。以下逐条覆盖 `visual-qa-checklist.md` 全部 27 项；pass 只表示注明的静态/自动化证据，未完成的视觉或实机部分明确保留 pending_human。

### 一致性

1. [x] **pass** — 同一 token 体系：`design-static.log` 的 COLORS 段中 `grep -rnE '#[0-9a-fA-F]{3,8}\\b|rgba?\\(' src/components src/App.tsx` 无命中；本 spec 四个组件的 RADII 扫描无硬编码 rounded-/borderRadius/boxShadow。新增 notice/footer/badge 使用现有 token。
2. [x] **pass（代码/计算样式）** — 强调色、圆角、字体沿用公共 CSS；隔离 E2E 对明暗背景、强调色、按钮与字体的计算样式断言通过。整体观感对照另见第 3 项。

### 主题与对比度

3. [ ] **pending_human** — 本 spec 新增文件动作/notice/结果反馈表面的明暗双模截图对照未执行。现有 E2E 有两主题计算样式断言，但不能替代关键状态截图验收；不声称已做截图比对。
4. [x] **pass（token 计算）** — `static-probe.log` WCAG sRGB 计算：浅色正文 11.22、muted 4.88、选中 9.40、主按钮 11.22、notice 6.45；深色分别 13.76、5.38、10.92、4.57、6.59，均 ≥4.5:1。选中/notice 背景按 alpha 合成 surface；未将未测的所有栅格/悬停状态算作通过。
5. [ ] **pending_human（部分自动化通过）** — 隔离 E2E 即时切换、移除过渡类、后端保存均通过；重启首帧、可感知白闪/跳变未做完整重启录像/人工观察。

### 字体与圆角

6. [x] **pass** — E2E fontProbe 实际加载 Playfair Display / Source Sans 3，随包 font-face 存在；src 中 Figtree 零引用（design-static.log）。
7. [x] **pass（本 spec 表面静态核对）** — window/card/row/pill 仍为 26/18/14/999 token；新增组件样式未引入规则外圆角；KWin 最终合成另列第 27 项。
8. [x] **pass（新增表面）** — 新增底栏、提示、徽标、操作反馈不使用中文斜体或小号衬线标题；本 spec 未改既有编辑器语法斜体规则。

### 动效与无障碍

9. [x] **pass（本 spec）** — 新增提示与底栏无额外动画，沿用列表反馈过渡；CSS 的快搜/管理时长满足 150/200/300 ms 上限，无卡片上浮。
10. [x] **pass（CSS/现有级联回归）** — 全局 reduced-motion 规则仍覆盖新增表面；E2E reduced-motion 级联探针通过（WebKit 无 media 模拟时以注入 media body 测量，不冒充真实系统设置切换）。
11. [ ] **pending_human（部分自动化通过）** — focus-visible 光环规则与键盘动作源码存在，E2E 核心导航/编辑/IME 通过；新增管理按钮的完整 Tab 顺序及真实键盘焦点可视巡检未完成。

### 状态完整性

12. [ ] **pending_human（部分自动化通过）** — 原有 loading/empty 结构仍在；新增损坏/缺失文件错误、导出结果路径在代码/E2E 有证据。全套 loading/empty/error 的明暗截图及“错误码+下一步”逐状态视觉检查未完成，不能以成功路径替代。
13. [x] **pass（代码/E2E）** — 缺失/自动导入标记保留；新增缺失动作失败路径和导出路径反馈由 E2E 覆盖；管理按钮缺失时禁用。文件真实外改另属人工验收。
14. [ ] **pending_human** — 保持三栏与 Q-A 结构，notice/footer 可换行；本次未做窗口尺寸下的信息密度及拥挤状态截图评估，VISUAL_DENSITY 6 的主观验收不伪报 pass。

### 隐私与安全（视觉侧）

15. [x] **pass（本 spec 回归/静态）** — 没有修改正文遮挡/编辑器挂载路径；quick row 不展示正文，既有敏感片段回归断言通过。未向测试记录写入真实用户正文。
16. [x] **pass** — FileTemplateDetail 无敏感开关，不展示文件内容；生成的 HTML 只流向 FormatClipboard，不通过 UI 渲染。
17. [x] **pass（新增路径审计）** — 新增复制/导出错误为固定文案；日志不输出模板名、路径、查询或正文。成功导出路径只显示给用户；E2E 剪贴板替身记录格式名/长度/哈希，不记录内容。

### 回归

18. [ ] **pending_human（部分自动化通过）** — Enter/Ctrl+Enter(autoPaste=false)/Shift+Enter/Ctrl+O/Ctrl+E/Esc/IME 等已有隔离断言通过；Ctrl+数字有分派与服务 FakeWm 测试，但本轮未实机验证数字动作的真实自动粘贴。真实自动粘贴保持 pending_human，不以 FakeWm 冒充。
19. [x] **pass** — autoPaste/exportDir 从 repository settings 读取，API 与 serde 类型保持对应；E2E settings_get/settings_update 持久化断言通过；无新增 localStorage 真相源。只读 exportDir 按人工接受项处理。

### Searchis 专项

20. [x] **pass** — src 中 backdrop-filter、surface-glass、canvas-glow 无引用（design-static.log）；E2E glass 探针通过。
21. [x] **pass（静态）** — 对照 design-tokens 表的双主题颜色、文件/语法色、字体、26/18/14/999、grain token；SPEC-23 CSS diff 只追加 notice/hints/file-badge 样式，未改 token 值。颗粒最终不透明度仍按规划留人工调定。
22. [x] **pass** — `static-probe.mjs` 静态 import 路径审计：src/components 下无未被 import 的 tsx 模块，HeaderBar 零引用；AboutModal 尚由 App import，删除属于 SPEC-24，不能在此 slice 要求提前移除。
23. [x] **pass** — 纸张颗粒层 pointer-events:none；隔离 E2E 对两主题 grainPointerEvents 断言通过；新增 notice 无拦截整窗覆盖层。
24. [ ] **repair：R1** — Q-A 标题、副标题、徽标、Ctrl+N 序号及匹配高亮保留，未置顶文件类型图标通过；但置顶文件图标被 Star 取代，不满足“文件按扩展名显示类型图标”。
25. [x] **pass** — 文件模板徽标用 fileExt.toUpperCase；文本仅 language≠plain 显示语言徽标；现有 E2E 分别覆盖 XLSX/文本语言。
26. [x] **pass（当前动作表面）** — 快搜无主按钮；管理新增两个动作均使用 secondary 样式，没有新增第二个实心主按钮；既有 E2E primaryCount 检查通过。
27. [ ] **pending_human** — 真实 KWin 合成器下 26px 圆角及 KDE Global Menu 五组菜单未验收；隔离 KWin 运行不能代替用户桌面结果。

## 实机验收

以下均未自动化，留待人在归档审批前记录于 `test_report.md`，不因这些 pending_human 本身要求 repair：

- pending_human：AC-30，真实剪贴板 md → WPS 文字。
- pending_human：AC-31，xlsx → WPS 表格及与原文件格式/布局截图对照。
- pending_human：AC-32，其他文件 → Dolphin 副本。
- pending_human：纯文本编辑器粘贴源码 / TSV / 路径。
- pending_human：真实自动粘贴、外部窗口激活、KDE 全局快捷键。
- pending_human：上列明暗截图、首帧/焦点巡检、KWin 圆角/Global Menu 与纹理调定。

## Next Step

只将 **R1** 返回 Implementer；修复置顶文件的类型图标并补回归断言后重新派发 Reviewer。不要因本报告修改 settings UI、备份、分组/扫描逻辑或设计工件。runtime 自行导入本报告及 result 推进状态；Reviewer 未标记 qa_passed、批准或归档。
