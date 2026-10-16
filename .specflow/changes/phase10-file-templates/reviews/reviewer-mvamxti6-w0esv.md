# QA Report: reviewer-mvamxti6-w0esv

## Verdict

- **`pass`**；`outcome: completed`。修复轮 #2 复审仅关闭上轮 **D1 / missing_test**，自动化回归未发现退化；R1、D2 保持上轮通过结论，不重开。
- 不改 state.yaml、不标记 QA passed、不批准或归档；由 runtime 导入本报告。未创建 subagent，源码只读。

## Scope / Inputs

已核对当前任务包、proposal.md、tasks.md、specs/file-templates.md、design.md、ux-design.md、contracts/22-file-template-management.md、项目宪法与构建配置、上轮 reviews/reviewer-mvamjozk-8fsj4.md、implementer-mvamr8kc-t3zz9.result.yaml。按用户补充仅审 D1 测试闭环及回归，不重做已接受的安全/架构裁定。

Implementer 本轮申报变更：src-tauri/src/commands.rs、e2e/tauri-service.mjs、e2e/create-and-search.e2e.ts。本轮精确复核这些位置及共用 importing 链，不把包含既有实现的工作区全量 diff 冒充本轮增量。

证据目录（下称 E/）：`reviews/evidence/reviewer-mvamxti6-w0esv/`，均在本任务允许路径内。

## Spec Coverage / Repair Verification

| 项目 | 证据 | 结论 |
| --- | --- | --- |
| D1 pending 反馈正断言 | E2E:879–902，测试先写 hold，再点击“导入文件”，成功/失败两轮均等待 import-progress 存在；失败轮另检查“导入中…” | 关闭 missing_test；不再仅用完成后不存在来证明 loading |
| D1 失败清理 | hold → pending 断言 → fail → 节点移除；错误提示“导入失败”、无新增文件 | 通过 |
| D1 成功清理 | hold → pending 断言 → release → 节点移除；后续导入 2 个、拒绝 1 个、原文件不变等原断言通过 | 通过 |
| 共用入口及 finally | App.tsx:501–522、687/692 与 ManagerWindow.tsx:602–604；picker 与拖入复用 handleImportPaths | 保持；无须重复两套端到端测试 |
| 闸门编译边界 | commands.rs:137 的调用点和 :144 的函数均为 cfg(feature = "e2e-webdriver")；Cargo 未声明 default feature，默认不启用 e2e-webdriver；非 E2E 二进制未检出环境变量/错误码/函数名 | 通过；生产默认构建不含该闸门 |
| 测试隔离 | tauri-service.mjs:14 在 SEARCHIS_E2E_FILES_DIR 下指定闸门文件；隔离脚本运行 Xvfb + 私有 D-Bus + KWin | 通过；未操作真实桌面 |
| R1 / D2 与 SPEC-21/22 回归 | 全量 Rust、构建及四场景 E2E 通过；原链接导入、安全拒绝、提示隐私断言保留 | 上轮通过结论保持，未发现回归 |

闸门读取 hold 时每 20ms 等待（上限约 30s），fail 返回注入错误，release/其他内容/文件缺失放行。不是依赖大文件或固定 sleep 争抢 loading 时序；本轮通过受控放行观察状态。E2E 断言通过后，额外非 E2E 构建覆盖默认二进制检查；没有将 debug 非 E2E 构建冒称发布包构建。

## Findings

- **D1 / missing_test：已关闭**。删除状态设置/节点将使 pending 正断言失败；仅完成后不存在的旧验证缺口已消除。
- 无新增阻断项、需求歧义或须人工裁决的架构/安全风险。目录级 TOCTOU 仍为已接受例外，不借复审重开。

## Tests Reviewed / Actual Results

| Command | 实际结果 | 日志 |
| --- | --- | --- |
| `cd src-tauri && cargo test` | exit 0；143 passed / 0 failed；main/doc 均 0 failed | E/cargo-test.log |
| `npm run build` | exit 0；tsc + Vite 完成；既有 >500kB chunk 提示 | E/frontend-build.log |
| `.specflow/bin/e2e-isolated.sh` | exit 0；4 passing，1 spec file passed；含快搜、分组、编辑历史、文件模板 | E/e2e-isolated.log |
| `cd src-tauri && cargo build --no-default-features --bin searchis` + strings 精确检查 | exit 0；非 E2E 二进制无 SEARCHIS_E2E_IMPORT_GATE、E2E_INJECTED_FAILURE、e2e_import_gate | E/production-boundary.log |
| 源码/设计静态检查 | 状态链、cfg、测试断言已核对；组件硬编码颜色/圆角、玻璃遗留 grep 均无匹配（exit 1） | E/source-review.log、E/design-recheck.log |

Rust 仍有既有 group_name dead_code；隔离 E2E 中有私有 D-Bus systemd1 激活失败及退出 KDE portal KCrash 等环境输出，但 WDIO 4 场景与脚本退出均成功。无失败截图需检查；不据此声称真实桌面 portal 或外部应用已验收。未直接运行真实 DISPLAY 上的 npm run test:e2e，未操作真实剪贴板、快捷键或自动粘贴。

## Design Conformance

**沿用上轮 27 条结论并逐条保留，仅 #12 更新为本轮验证通过。其余条目中的“本轮”、原行号和旧日志名保留为上轮历史证据，不表示本次重测；本次新增证据见 E/。** 本轮额外重跑 token/玻璃检查与自动化回归，未发现使旧结论失效的变更；所有 pending_human 保持待人验。

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

12. [x] **loading/empty/error（静态/自动化）**：本轮 D1 missing_test 已关闭。App.tsx:501–522 的共用 handleImportPaths 在 await 前置 importing、finally 清除；ManagerWindow.tsx:602–604 在 importing 时挂载带 role=status 的 import-progress“导入中…”，该类无隐藏样式。E2E:879–902 通过 hold 保持 pending，成功与失败两轮均先断言状态节点存在，失败轮还断言“导入中…”文案；写入 fail/release 后均断言节点移除，失败另检查错误提示与无副本，成功沿用导入计数/原文件不变断言。这里是 DOM/状态的自动化可见性证据，不冒称像素、辅助技术或双主题截图验收；其他 empty/error 结论沿用。
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

- AC-24 KDE/Dolphin 真实拖入、AC-26 Dolphin 改名移动、AC-27 WPS 保存及 WPS/LibreOffice 打开方式仍 pending_human；tasks 3.13 不勾选。
- #3/4/5/6/11/14/18/27 的明暗截图、完整渲染对比度、字体加载、首帧、焦点/IME/快捷键、信息密度、KWin 圆角与 Global Menu 均保留 pending_human。
- 文件快搜交互/导出留 SPEC-23，完整设置留 SPEC-24，已接受目录级 TOCTOU 不重开。

## Next Step

由运行时导入 `pass` 复审报告及 result 并决定后续状态；本 Reviewer 不推进状态、不批准或归档。归档前仍需人工完成上述验收。
