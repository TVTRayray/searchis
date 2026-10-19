# QA Report: reviewer-mv98vwtn-tluwx

## Verdict

`escalation` — category: `permission_insufficient`；result outcome: `needs_decision`。

仅审查 visual-foundation / SPEC-20，未修改业务代码，未推进 state、批准或归档。读取指定任务包、五项输入、contract 20、设计宪章/tokens/视觉清单、上一轮 QA、最新 implementer result 与工作区真实 diff。发现契约范围冲突后停止进一步审查与执行测试；本报告不是完整复审或 QA 通过。

## Spec Coverage

| Requirement / scenario | Evidence | Result |
|---|---|---|
| 无玻璃残留与组件无硬编码颜色 | 本轮两条机械 grep 无结果；main=false 的配置 diff | 静态检查通过，不代表完整视觉验收 |
| F1 快搜焦点环 | command.tsx 增加 quick-search-field；index.css 的 :focus-within 声明 2px、offset 2px、focus-ring token | 修复代码存在；本轮未运行动态断言 |
| F2 reduced-motion | index.css 增加主题过渡覆盖；ThemeProvider.tsx 增加 matchMedia 条件 | 存在权限范围问题 S1；不作通过结论 |
| F3 placeholder 对比度 | index.css 使用 secondary；contrast-report.md 更新配对；最新实现 result 报告 6.69/7.25、7.70/8.31 | 代码与实现证据已阅；本轮未独立复算或运行探针 |
| 字体、形状、主题切换、HeaderBar、旧设置、其余对比度 | 实现源码、前轮报告与最新 result | 本轮完整复审因 S1 中止，不以历史通过替代本轮结论 |

## Findings

### S1 — reduced-motion 修复超出 ThemeProvider 的契约内修改权限

- Severity: medium（流程/授权阻塞，并非认定修复本身有功能缺陷）
- Category: `permission_insufficient`
- Expected: contract 20「允许修改范围」明确限定 `src/components/ThemeProvider.tsx` **仅主题展示名**；项目规则规定 contracts 的“仅限”约束同样有效。修复轮不能以功能验收要求自动扩张允许编辑内容。
- Actual: `git diff -- src/components/ThemeProvider.tsx` 显示，原本无条件添加 theme-transition class 的代码被改为仅在 `!window.matchMedia('(prefers-reduced-motion: reduce)').matches` 时添加。这是运行逻辑变更，不是主题名称。最新 implementer 包列出该文件，但没有解除 contract 的内容限制；前轮 F2 建议不启用过渡也不能替代授权。
- Reproduction / evidence: 对照 `contracts/20-botanical-ember-visual-foundation.md:23` 与 `src/components/ThemeProvider.tsx:72-74`；最新实现 result 自述该 guard 是 F2 的“双保险”。CSS 本身另有 reduced-motion 覆盖，未发现必须借此扩张权限的理由。
- Required decision: Orchestrator/人确认是否授权此处极小的 reduced-motion guard，并由 Planner 同步契约范围后重派；或要求 Implementer 只撤去 guard、保留 CSS 修复与回归探针，再运行 build/E2E。Reviewer 不自行回退代码或修改契约。

## Tests Reviewed

本轮实际执行：

- `git diff --check`：无输出，静态空白检查通过。
- `git diff --name-only -- src-tauri/src src/api src/types src/lib`：无结果；无 Rust 业务/API/types/lib diff。
- `grep -rnE 'backdrop-filter|glass|canvas-glow|Figtree|HeaderBar' src/`：无结果（预期 exit 1）。
- `grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components src/App.tsx`：无结果（预期 exit 1）。此检查不证明圆角等其他属性全部合规。
- Rust scoped test：not_applicable，无 Rust 逻辑变更。
- `npm run build` / `npm run test:e2e`：本轮未执行，发现 S1 后按指令停止。

转引而非本轮验证：`handoffs/implementer-mv98o4zo-bexgr.result.yaml` 报告最终 build、E2E（1 passing，6.3s）通过，并报告 F2 负向验证按预期失败。新测试断言代码已阅，但不能作为本轮亲自运行的通过证据。E2E 含 xdotool 前台窗口激活/输入，后续执行仍须遵守真实前台输入授权边界。

## Design Conformance

逐项对应 `docs/design/visual-qa-checklist.md`；未勾选表示未完成/不适用，不隐含产品缺陷。权限阻塞前的机械检查不替代完整设计复审。

### 一致性
- [x] 组件/App 硬编码颜色 grep 无结果；圆角完整检查未执行。
- [ ] 强调色、圆角、字体全应用一致：完整复审中止。

### 主题与对比度
- [ ] 明暗关键面截图：pending_human。
- [ ] 正文/muted/选中/主按钮对比度：已读实现报告，本轮未独立复算。
- [ ] 主题切换无白闪、重启首帧正确：pending_human；本轮未执行动态探针。

### 字体与圆角
- [ ] 字体真实加载：本轮未运行；Figtree 源码零引用已机械确认。
- [ ] 圆角规则全应用：完整复审中止，KWin 圆角 pending_human。
- [ ] 中文无斜体、衬线仅 ≥18px：本轮未完成全量核对。

### 动效与无障碍
- [ ] 动效上限：未运行动态探针。
- [ ] reduced-motion 全部瞬时：S1 阻塞；不作通过结论。
- [ ] 焦点可见、全键盘可操作：F1 修复源码存在，运行回归未执行。

### 状态完整性
- [ ] loading/empty/error：完整复审中止；扫描/导入/分组/文件筛选属于后续 spec。
- [ ] 文件缺失、自动导入、导出路径：not_applicable，后续 spec。
- [ ] 信息密度：实机对照 pending_human。

### 隐私与安全（视觉侧）
- [ ] 敏感遮挡与未挂载正文：本轮未完成动态回归。
- [ ] 文件模板不显示敏感开关/预览：not_applicable，后续 spec。
- [ ] 日志/提示隐私：本轮未完成专项审查；Rust/API/types/lib 无 diff。

### 回归
- [ ] 键盘及 IME 语义：未运行，实机回归 pending_human。
- [ ] 设置持久化后端契约：本轮未动态验证，未改存储协议。

### Searchis 专项
- [x] 玻璃/氛围光/HeaderBar 源码 grep 零结果。
- [ ] token 逐项一致：本轮未完成独立全表比较。
- [ ] 未引用组件全量核对：未执行；HeaderBar 零引用已确认，AboutModal 仍属现阶段 UI。
- [ ] 颗粒不拦截交互：未运行探针；实机强度 pending_human。
- [ ] Q-A 结果行：not_applicable，groups-language。
- [ ] 徽标规则：not_applicable，后续 spec。
- [ ] 每界面最多一个主按钮：本轮未完成动态探针。
- [ ] KWin 26px 圆角、KDE Global Menu 五组菜单：pending_human。

## Next Step

停止，等待 Orchestrator/人对 S1 作决策。不自行更新 state 或把 spec 标为 qa_passed。待授权范围一致或撤去越界 guard 后再复审 F1–F3，并执行获准的验证。浅/深各六张截图、Global Menu、颗粒调定、首帧无白闪等仍为 pending_human，由人在归档前记录于 test_report.md。
