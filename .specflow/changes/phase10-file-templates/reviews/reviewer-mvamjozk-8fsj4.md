# QA Report: reviewer-mvamjozk-8fsj4

## Verdict

- **`repair` / `missing_test`**；修复轮 #1 后复审完成，`outcome: completed` 仅表示 Reviewer 工作完成，不代表实现验收通过。
- **R1：通过；D2：通过；D1：实现已修复，但 Required repair 的延迟响应状态断言未补，不能关闭。**
- 无新增需求/架构/安全裁决请求。不改 state.yaml、不标记 QA passed、不批准或归档、不创建 subagent。

## Scope / Inputs

已核对本任务包、proposal.md、tasks.md、specs/file-templates.md、design.md、ux-design.md、contracts/22-file-template-management.md、state.yaml、上轮 reviews/reviewer-mvam3u2u-ho1u1.md 及本轮 implementer-mvamh1kn-i144m.result.yaml。本轮只复核 R1/D1/D2 与其影响面，非重开所有既有验收。

执行边界：按 Planner 裁定及用户补充，根外导入源解析链接后复制；根内移动继续 O_NOFOLLOW/EXDEV 拒绝。目录级 TOCTOU 是已接受例外，不重开；QuickSearchWindow.tsx 的文件模板展示属于 SPEC-23，不作为本轮缺陷。源码只读，修改仅在本 change 路径内。

本轮日志目录（下称 E/）：`reviews/evidence/reviewer-mvamjozk-8fsj4/`。上轮静态审计及待人工项沿用原报告，未声称本轮重复了完整视觉验收。

## Spec Coverage / Repair Verification

| 项目 | 源码与测试证据 | 本轮结论 |
| --- | --- | --- |
| R1 根外链接导入 | service/templates.rs:437–443 canonicalize 后将解析路径传给 import_one；命名仍取所选 source。:1148–1163 回归检查内容、所选链接名、链接及目标不变、返回记录不带源路径；cargo template 实际通过 | 已按裁定修复 |
| 保留根内拒绝策略 | templates/fsops.rs:92–104 仍以 O_NOFOLLOW 打开复制源/目标；:155–173 对链接 EXDEV 拒绝；group_delete_with_symlink_entry_on_exdev_is_rejected_and_fully_rolled_back、copy_with_hash_rejects_symlink_source 等在本轮 cargo test 中通过 | 未发现该策略退化 |
| D1 导入状态实现 | App.tsx:202、501–522、687/692 与 ManagerWindow.tsx:602–604：选择文件与拖入共用 handleImportPaths，await 前 setImporting(true)，finally 清除，role=status 显示“导入中…” | 实现部分修复；测试部分未满足，见 Findings |
| D2 提示隐私 | App.tsx:508–518、543、556 去除 rejected.name/created.title/saved.title 的插值；E2E:880–883 检查明文说明、计数、不出现被拒绝文件名 | Required repair 满足；卡片/表单显示保持 |
| 既有 SPEC-22 / SPEC-21 回归 | 全量 Rust 143 passed；隔离 E2E 四场景通过（快搜基线、分组、编辑历史、文件模板主路径） | 自动化回归通过；不代替人验 |

Implementer 本轮申报的改动文件为 `src-tauri/src/service/templates.rs`、`src/App.tsx`、`src/components/ManagerWindow.tsx`、`e2e/create-and-search.e2e.ts`；本轮逐处只读复核。当前工作区包含既有未提交实现，不将全量 git diff 冒充本修复轮增量。

## Findings

### D1（延续）— 未补延迟响应期间可见的状态断言

- Severity: medium；Category: `missing_test`。这是上轮 D1 未完成部分，不是新增需求或新的实现缺陷。
- Expected: 上轮 Required repair 明确要求“为选择文件与拖入共用的导入流程增加最小的进行中反馈，结束/失败均可靠清除；补一个延迟响应时的状态断言”。第 12 条另要求覆盖失败清理。
- Actual: 源码已有状态实现，但 E2E:878–883 先等导入完成及记录出现，再断言没有“导入中…”。未控制响应延迟，也没有导入 pending 时为可见的正断言。Implementer 的 unresolved_issues 自述与源码一致。
- Evidence / falsifiability: 如果删除 setImporting(true) 或整个状态节点，当前新增的“完成后不存在”断言仍可通过，因此不能证明 loading 曾显示。`E/source-review.log` 保存对应源码及测试行；本轮 4 passing 也不能补足这一缺口。本轮未进行越权源码变异实验。
- Required repair: **仅补 D1 的确定性回归测试**。使用测试侧可控的 deferred/延迟响应，让共用导入流程确实停留在 pending；先断言 `data-testid=import-progress` 可见（或 role=status 对应文案可见），再释放响应并断言成功后消失；另覆盖拒绝/失败后清除。避免只靠 sleep/大文件碰时序；无需新增生产任务系统，也不要求分别重复 picker 与拖入全流程。若所用设施需要超出任务授权路径，请交运行时决策，不由 Reviewer 改源代码。

## Tests Reviewed / Actual Results

| Command | 实际结果 | 完整日志 |
| --- | --- | --- |
| `cd src-tauri && cargo test template` | exit 0；56 passed，0 failed | E/cargo-template.log |
| `cd src-tauri && cargo test` | exit 0；143 passed，0 failed；main/doc tests 均 0 failed | E/cargo-test.log |
| `npm run build` | exit 0；tsc + Vite 成功；仅 >500kB chunk 提示 | E/npm-build.log |
| `.specflow/bin/e2e-isolated.sh` | exit 0；4 passing，1 spec file passed；Xvfb + 私有 D-Bus + KWin | E/e2e-isolated.log |
| 精确源码/token/日志检查 | 组件颜色、圆角与玻璃 grep 无匹配（exit 1 表示无匹配）；相关提示与状态链复核完成 | E/source-review.log、E/design-recheck.log |

环境提示：Rust 既有 group_name dead_code；WDIO Debian 包名诊断、私有 D-Bus systemd1 激活失败、退出时 KDE portal KCrash。测试 exit 0，不据此声称真实桌面 portal 已验收。无失败截图需要检查；未操作真实桌面快捷键、剪贴板、自动粘贴或外部应用。

## Design Conformance

**沿用上轮完整 27 条结果，逐条保留结论；本轮重点复核 #12（状态完整性）、#17（提示隐私），并重跑 token/玻璃静态检查及 E2E 回归。** 以下“上轮”及原行号是历史证据，非本轮重新测量；不受本轮修改影响的项目沿用，不把 pending_human 改成通过。`[x]` 仅为标明范围的静态/自动化结论。

### 一致性

1. [x] **统一 token 体系**：精确 `grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components src/App.tsx` 无匹配；另查 rounded 工具类与 inline borderRadius 均无匹配。SPEC-22 CSS 使用语义色、radius、shadow token。见 grep-design.log/design-audit.log。
2. [x] **强调色/圆角/字体同体系（静态）**：新文件卡片、弹出菜单、徽标、对话框均复用已有 manager 类；无第二套主题或私有字体。跨窗口渲染观感在第 3 条待人验。

### 主题与对比度

3. [ ] **pending_human：明暗关键面截图**。上轮 E2E 是 DOM/状态断言，不是双主题成功截图对照；需浅/深文件列表、详情、打开方式、缺失、导入提示与预览稿并排核验。
4. [ ] **pending_human：完整渲染态对比度**。已重新计算 token 配色，见 design-audit.log：浅/深正文在 surface 为 11.22/13.76，secondary 7.70/8.31，muted 4.88/5.38；主按钮 11.22/4.57，hover 4.74/5.07；选中态主/secondary 均超过 4.5。不能把这视为任意 token 组合都通过：浅色 muted/sidebar=4.24、muted/canvas=4.47，选中底叠加后的某些 muted 配对也低于 4.5，须核验实际渲染是否使用这些配对（禁用/placeholder 另计）。上轮不根据不存在的组合虚报新缺陷，也不宣称全矩阵 AA；目标界面实际配对需实机复核并记录。
5. [ ] **pending_human：主题切换与重启首帧**。ThemeProvider 经 settings 后端设置主题，CSS 两套色；现有 E2E 有主题探针但不证明无首帧白闪、每个模板状态无跳变，需重启与连续切换目视验收。

### 字体与圆角

6. [ ] **pending_human：字体真实加载**。静态通过：10 个 @font-face 资源全部存在、构建打包字体，Figtree 零引用。真实字体非回退（包括中文 Noto）仍须运行时字体检查，不能用文件存在替代实际加载。
7. [x] **圆角（本 spec 静态）**：新模板 CSS 全用 card=18、row=14、pill=999，窗口=26；颗粒 inherit。全 CSS 另有既有 `.quick-search-window mark` 3px（index.css:1034）未满足四档规则，明确记录为 SPEC-21/快搜基线例外，不算本次新增、也不要求上轮实现者越界改 QuickSearchWindow。KWin 实际窗口见第 27 条。
8. [x] **中文无斜体、衬线 ≥18px（本 spec 静态）**：新建错误 em 的 manager-field-error 强制 font-style:normal；模板拖入标题为 18px 衬线，其余新增文本用正文。@font-face 的 italic 声明不是中文元素使用斜体；不将声明误报。

### 动效与无障碍

9. [x] **动效动机/时限（静态）**：quick=120ms、popup=180ms、manager=220ms，分别用于选中/弹出/悬停反馈。is-spinning=1s 是扫描持续反馈循环，不是一次交互响应延迟；未新增列表上浮。
10. [x] **reduced-motion（静态）**：全局覆盖 duration=0.01ms，theme-transition/animate-pop-in 强制 none，新增 is-spinning 在 reduce 下 none。无新增 JS 动画绕开该守卫；运行时偏好切换可随实机项复核。
11. [ ] **pending_human：全键盘焦点路径**。CSS focus-visible/within 为 2px outline + 2px offset；菜单有 role/aria、初始焦点与 Esc，对话框有初始焦点；隔离 E2E 有基础键盘回归。但不证明全 Tab/Shift+Tab、关闭恢复焦点、打开方式菜单和不同输入法路径全程可用，需人工走完；不冒称 E2E 覆盖所有辅助技术。

### 状态完整性

12. [ ] **fail（验证缺口）：loading/empty/error**。本轮复核 App.tsx:501–522、ManagerWindow.tsx:602–604，已增加共用 importing 状态与可见 role=status，finally 覆盖成功/失败清理；原“缺导入状态”的实现问题已修复。现有 E2E:883 只在导入完成后检查文案不存在，未断言延迟响应期间可见，也未覆盖失败后清除，故 D1 的 Required repair 尚未闭环（见下方 D1，missing_test）。其他 empty/error 的上轮结论沿用；双主题外观继续 pending_human。
13. [x] **缺失/自动导入/导出路径状态（范围化）**：FileTemplateDetail 有“文件缺失”及放回后重扫指引、自动导入标记；E2E 验证导入标记及保存清除，Rust 验证缺失/恢复。导出按钮按 SPEC-22 禁用并提示后续版本；**实际导出路径提示属 SPEC-23，本 spec 不适用**，不是遗漏实现。
14. [ ] **pending_human：信息密度 6**。静态仍是三栏、紧凑列表/文件信息卡片，无内容预览；需第 3 条实际截图检查中文截断、换行、菜单遮挡与视窗密度，不能从 DOM 数据断言推导审美通过。

### 隐私与安全（视觉侧）

15. [x] **敏感正文遮挡（静态）**：ManagerWindow:787 起为 isSensitive && !sensitiveRevealed 条件分支；遮挡时不挂载 CodeEditor。SPEC-22 没有改为将文件内容作为敏感文本编辑。真实会话/辅助技术的视觉体验仍可人工回归。
16. [x] **文件无敏感开关/不预览**：FileTemplateDetail 仅元数据、文件信息与动作，无内容加载或渲染；上轮 E2E 检查详情禁用动作和无敏感输入。
17. [x] **日志/提示隐私（本 spec、静态/自动化）**。本轮复核 App.tsx:508–518、543、556：导入提示仅计数/原因，新建仅模板类型与明文说明，保存不插入标题/文件名/路径；元数据卡片/表单正常显示不受此限。E2E:880–883 实测明文说明、成功/失败计数、不出现被拒绝文件名。模板模块 grep 仅发现测试性能 eprintln，无生产路径/名称日志。D2 已修复；不把全应用既有 key/文本提示纳入本次清理。

### 回归

18. [ ] **pending_human：完整快捷键/IME 矩阵**。上轮 E2E 的快搜视觉/键盘与编辑历史回归通过，但不等于所有 Enter/Ctrl+Enter/Shift+Enter/Ctrl+O/Ctrl+E/Ctrl+数字/Esc 与真实 IME 全组合验收。文件快搜按键属 SPEC-23；全局快捷键/自动粘贴明确只留人验，未自动操作。
19. [x] **后端设置真相源（静态/既有 E2E）**：ThemeProvider 从 settingsApi.get/patch 同步，App 的 open-with 由模板后端命令记忆；没有新增以 localStorage 替代后端的业务状态。完整设置视图/备份属 SPEC-24。

### Searchis 专项

20. [x] **去玻璃**：src/ 中 backdrop-filter、surface-glass、canvas-glow 零匹配；未新增玻璃表面。拖入虚线 overlay 的 color-mix 是临时状态覆盖，不是常驻玻璃面板。
21. [x] **token 文档逐项静态对照**：design-audit.log 输出两主题全部 color/font/radius/motion/grain 值；与 design-tokens.md 逐项复核，显式列出的画布、surface、侧/列表/输入/代码/底栏、三级文字/disabled、边框、强调、主/次按钮、选中、图标底、focus/highlight、语义色、文件/语法色、四圆角、三字体一致（rgba 空格视为同值）。文档省略的阴影 alpha 序列不是精确数值合同；grain=0.06/0.035 尚待人工调定，不伪称最终实机值。
22. [x] **组件引用清单**：审计 16 个有运行时导出的组件模块，每个至少一条静态 import（见 design-audit.log）；HeaderBar 零引用。AboutModal 当前仍被 App 使用，其删除属于 SPEC-24，不能在此把它误当已删除死代码。模块引用不冒称每个命名导出均已 tree-shaking 分析。
23. [x] **颗粒不挡输入（静态）**：main-window-shell/quick-search-window ::after 明确 pointer-events:none；drop overlay 也为 none。真实 KDE 拖放事件传递仍属于 AC-24 实机回归，未用代码检查冒充 Dolphin 拖入实测。
24. [x] **Q-A 基线（自动化/范围化）**：快搜既有 Q-A 视觉/键盘 E2E 上轮通过；文本行维持类型图标、标题、Key/分组/次数、语言徽标、Ctrl 序号、高亮代码路径。文件行图标/徽标/按键由 SPEC-23 接续，不以这部分过渡态阻断 SPEC-22。
25. [x] **徽标规则（静态/自动化）**：管理文件行与详情用 ext.toUpperCase，FileTypeIcon 按文档/表格/pdf/代码分类；文本 language≠plain 才显示语言，SPEC-21 E2E 回归通过。快搜文件行延期范围同第 24 条。
26. [x] **最多一个主按钮（本 spec 静态）**：文件详情“打开”为主，其余（含保存）secondary；NewTemplateDialog 只有“创建”为主，PlusMenu/打开方式菜单无主按钮；打开创建对话框遮挡背景，非新增并列主动作。全屏各态视觉层级随第 3 条确认。
27. [ ] **pending_human：KWin 圆角/Global Menu**。隔离 KWin 中 E2E 通过不证明用户真实合成器下 26px 外轮廓和 KDE 五组 Global Menu 正常；必须由人操作真实桌面验证。

## Pending Human / Non-blocking Boundaries

- 沿用上轮：AC-24 真实 KDE/Dolphin 拖入、AC-26 Dolphin 改名移动、AC-27 WPS 原子保存、WPS/LibreOffice 实际打开方式，均待人工；本轮未启动外部应用冒充人验。
- 明暗截图/完整对比度、字体真实加载、首帧与主题切换、信息密度、完整焦点/IME/快捷键矩阵、KWin 外轮廓和 Global Menu 仍 pending_human。
- 目录级 TOCTOU 维持已接受例外；文件快搜交互与导出留 SPEC-23，完整设置留 SPEC-24。无本轮新增升级项。

## Next Step

运行时仅将 **D1 的 missing_test** 退回 Implementer；R1、D2 不再重开。补齐可控延迟可见性与成功/失败清理断言后复审。Reviewer 不推进 state.yaml 或标记 QA passed。
