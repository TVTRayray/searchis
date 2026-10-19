# QA Report: reviewer-mvam3u2u-ho1u1

## Verdict

- **`repair` / `implementation_defect`**；本次 reviewer 工作已完成，`outcome: completed`，不等于实现验收通过。
- 上轮 **S2 按人工决策 A（拒绝策略）已修复**，不再升级安全决策。根外导入源链接被同一保护误拒绝是新的实现回归，按本轮 Planner 裁定列 R1。
- 完整设计清单已逐项处理：静态通过、发现的问题、`pending_human` 分开记录；没有把尚未实测的视觉项写成通过。
- 仅写本 change 的报告、结果和证据；未改产品源码、state.yaml、tasks.md、test_report.md；未创建 subagent，未自行标记 QA passed、批准或归档。

## Inputs / Scope

已读任务包、proposal.md、tasks.md、specs/file-templates.md、design.md、ux-design.md、contracts/22-file-template-management.md、项目宪法、上轮 reviewer-mvalps6a-rgrqq.md 及 implementer-mvalyck7-wzbvo.result.yaml。设计核对依据 docs/design/{design-charter,design-tokens,visual-qa-checklist}.md。

本轮遵循 Planner 转达裁定：用户主动选择/拖入的根外链接应解析并复制目标内容；根内移动/扫描的链接防护保留。目录级 TOCTOU 是 contracts/22 已接受例外；QuickSearchWindow.tsx 的文件模板展示属于 SPEC-23。不触发 KDE 全局快捷键、真实剪贴板、自动粘贴或真实桌面前台输入。

证据目录（下文简称 `E/`）：`reviews/evidence-reviewer-mvam3u2u-ho1u1/`（相对此 change）。

## S2 专项与 implementer unresolved_issues 核对

1. `fsops::move_file` 在移动前以 `symlink_metadata` 识别源类型；同设备 rename 移动链接本身，EXDEV 且源是链接时返回 `TEMPLATE_FS_FAILED`，不进入复制回退。`copy_with_hash` 的源端另有 `O_NOFOLLOW`。
2. 新增的 `group_delete_with_symlink_entry_on_exdev_is_rejected_and_fully_rolled_back` 测试通过：验证链接类型、普通文件/目录/数据库回滚、无外部哨兵实体化、目标未变；helper 两个拒绝链接测试也通过。
3. 本轮另用从当前源码原样抽取的 helper，在 `/tmp` 与 `/dev/shm` 的临时目录验证**真实 EXDEV**，不是测试钩子：操作被拒绝、链接保留、目标不存在、外部合成哨兵未变。见 `E/make-repro.py`、`s2-import-repro.rs`、`s2-import-repro.log`。只简化 AppError，不改函数控制流；这不是整个 group_delete 的真实跨设备端到端测试。
4. 结论：原 S2 的静态链接跟随读取已修复；不重新质疑已接受的目录级 TOCTOU。

| implementer unresolved_issues | 本轮判断 |
| --- | --- |
| 导入源链接因 O_NOFOLLOW 被拒绝，声称是预期防御 | **不同意其“预期”结论**。按 Planner 已明确的根外输入契约，这是 R1 / implementation_defect，不是 requirements_conflict、security_risk 或新的需求决策。 |
| EXDEV 仅 cfg(test) 钩子模拟 | 服务级测试确为钩子；本轮补了真实 EXDEV helper 证据，未冒称完整业务跨设备 E2E。 |
| 已复制跨设备普通文件的逆向 rename 可能失败，产生 TEMPLATE_RECONCILE_PENDING | 这是既有补偿失败收敛路径，不是此次链接防护新增行为；契约允许记录并下次扫描收敛。本轮不宣称所有跨设备补偿都原子成功。 |
| 目录级 TOCTOU 已接受、3.13 与设计实机项待验 | 接受前者的既定例外；后者在本报告逐项列 pending_human，不将它们伪记通过。 |

## Spec Coverage

| Requirement / scenario | Evidence | Result |
| --- | --- | --- |
| 模板根/.trash 权限、已有分组补目录、分组创建/重命名/移动/删除同步 | startup/private permissions、group_directories_follow_*、deleting_a_group_*、failed_group_delete_* Rust 测试 | 自动化通过 |
| v4→v5、备份、字段约束与路径唯一 | migrates_v4_to_v5_keeping_data_and_writing_backup、v5_constraints_* | 自动化通过 |
| 普通文件导入、原件不变、Key/自动导入标记、大小/容量/多文件部分成功、新建 md/xlsx | import_*、create_blank_md_and_xlsx_*；SPEC-22 隔离 E2E | 既有测试通过；根外链接导入不通过，见 R1 |
| 文件详情、改标题同步重名后缀、按分组移动 | rename_title_*、update_moves_file_*；E2E 碰撞改名及清除自动导入标记 | 自动化通过 |
| xdg-open 替身、缺失错误、MIME/.desktop/Exec 与应用排序 | platform::files::*、open_reports_missing_file_*；E2E open-with 菜单/调用替身 | 自动化通过；真实 WPS/LibreOffice 待人验 |
| 路径→inode→唯一哈希、原子保存、外部改名/移动、缺失/返回、新目录/深度/忽略/链接/并发扫描 | templates::scan::*、service::templates::scan_*；E2E 外部改名重扫 | 自动化通过 |
| 回收/恢复/彻底删除/清空/自动清理、索引同步 | trash_lifecycle_*、empty_trash_*、restore_*、purging_a_missing_entry_*；E2E 生命周期 | 自动化通过 |
| 先文件后 DB、失败补偿 | import_database_failure_*、update_moves_file_*、failed_group_delete_* | 自动化通过；不等于覆盖一切 OS 故障 |
| 文件检索字段、文本复制拒绝与不计数 | generic_snippet_update_and_copy_reject_file_templates；E2E 文件检索、移出索引与复制拒绝 | 自动化通过；快搜文件行视觉归 SPEC-23 |
| 静态链接/路径遍历防护 | paths::*、symlinked_intermediate_directory_*、symlinked_group_directory_*、本轮 S2 复现 | 根内拒绝策略通过；不应套用于用户导入源 |
| 2,000 文件无变化扫描性能 | qa_perf_unchanged_scan_of_2000_files 在本轮 cargo test 中通过（测试内断言 ≤2s） | 当前测试机通过，不冒称目标机实测 |

## Findings

### R1 — 根外用户选择的链接导入被误拒绝

- Severity: medium；Category: **`implementation_defect`**。
- Expected: 导入用户主动指定的文件链接应解析源并复制目标内容，保持链接与目标不变，不持久化源路径；根内移动的 `O_NOFOLLOW` 和 EXDEV 拒绝策略保持有效。
- Actual: `src-tauri/src/service/templates.rs:390–443` 以 `fs::metadata` 跟随源进行文件类型/大小检查，却把原始 `source_path` 传入 `import_one`；`:478` 再原样交给 `fsops::copy_with_hash`。`src-tauri/src/templates/fsops.rs:92–98` 源端 `O_NOFOLLOW` 导致叶子链接返回 ELOOP，转为 `TEMPLATE_FS_FAILED / 复制文件失败`。
- Reproduction: `E/s2-import-repro.log` 显示同一 helper 对合成普通源成功、对指向该源的根外链接返回 ELOOP，链接本身未变；本轮静态调用链证明 import 没有单独解析源。不是仅引用 implementer 的自述，也不冒称新增了产品集成测试。
- Required repair: 对导入源单独处理，例如 canonicalize 后验证为普通文件、检查大小，再复制解析后的目标；保持命名/扩展名、原文件不动和导入原有约束。不要全局移除 `O_NOFOLLOW` 或放松根内路径防护。补服务级回归：**从根外链接导入成功；导入内容与目标一致；链接仍是原链接；目标不变；源路径不入库**。保留 S2 拒绝及回滚测试。

### D1 — 导入过程缺少 loading 状态

- Severity: medium；Category: `design_conformance`。
- Expected: visual-qa-checklist「loading（扫描中、导入中）」与 ux-design 的状态完整性要求：导入未结束时给出可见的进行中反馈。
- Actual/evidence: `src/App.tsx:500–517 handleImportPaths` 直接 await import/refresh，只在完成/失败后 toast；没有设置导入中状态。`ManagerWindow` 只有 `scanning` 属性和扫描中反馈；PlusMenu 也没有导入 busy/disabled 输入。NewTemplateDialog 的创建 busy 不等于文件导入 loading。
- Required repair: 为选择文件与拖入共用的导入流程增加最小的进行中反馈，结束/失败均可靠清除；补一个延迟响应时的状态断言。无需引入额外任务系统。此结论来自状态链源码缺失，不是凭成功截图推断，也没有伪造失败截图。

### D2 — 模板动作 toast 带文件名/标题，不符合提示隐私清单

- Severity: low；Category: `design_conformance`（不升级为安全漏洞）。
- Expected: visual-qa-checklist「日志/提示不含查询词、正文、模板文件名或路径（导出成功路径除外）」。元数据卡片按契约显示文件名不受此项禁止。
- Actual/evidence: `src/App.tsx:506–510` 把 rejected.name 拼进导入 toast；`:541` 新建 toast 包含 created.title 与扩展名；`:555` 改名保存 toast 包含 saved.title。后台模板实现未发现生产日志输出这些值，但瞬态提示仍不符合清单。IPC 的 rejected.name 字段可以保留，不要求改接口或文件详情显示。
- Required repair: 本 spec 新增的模板动作 toast 改为成功/失败计数、错误码与下一步，不拼接模板文件名/路径；保留卡片/表单正常显示。补针对提示文案的断言即可，不扩展为全应用文案清理。

## Tests Reviewed / Actual Results

均为本轮实际运行，完整输出存 E/：

| Command | Result / evidence |
| --- | --- |
| `cd src-tauri && cargo test template` | 55 passed，0 failed；cargo-template.log |
| `cd src-tauri && cargo test` | 142 passed，0 failed；main/doc tests 各 0 failed；cargo-test.log |
| `npm run build` | exit 0，tsc + Vite 成功；npm-build.log。仅 >500kB chunk 提示。 |
| `.specflow/bin/e2e-isolated.sh` | exit 0，4 passing、1 spec file passed；e2e-isolated.log。包含 SPEC-21、编辑历史修复、SPEC-22 主路径。只由脚本在 Xvfb + 私有 D-Bus + KWin 中调用 test:e2e，未直接运行真实 DISPLAY 的 npm test:e2e。 |
| 当前 fsops 抽取复现（rustc 编译后执行） | 真实 EXDEV 拒绝链接；根外链接复制 ELOOP；普通源对照成功。s2-import-repro.log |
| `/home/ray/.venvs/python-scripts/bin/python E/design-audit.py`、精确 grep | 执行成功；design-audit.log、grep-design.log。后者颜色与 backdrop grep exit 1 是无匹配，不是失败。 |

环境诊断：Rust group_name dead_code、WDIO 的 Debian 包名检测警告、私有 D-Bus systemd1 激活失败及 KDE portal KCrash；测试仍 exit 0，不据此宣称真实桌面 portal 正常。所有生成的临时哨兵和复现二进制已清理。

## Design Conformance

**27 条全部列出。** `[x]` 表示所注明范围的代码/自动化检查通过，非完整像素视觉验收；`[ ] pending_human` 有明确待验内容；`[ ] fail` 对应实际发现。未运行成功截图人工视觉验收，不假借前轮结果通过。

### 一致性

1. [x] **统一 token 体系**：精确 `grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components src/App.tsx` 无匹配；另查 rounded 工具类与 inline borderRadius 均无匹配。SPEC-22 CSS 使用语义色、radius、shadow token。见 grep-design.log/design-audit.log。
2. [x] **强调色/圆角/字体同体系（静态）**：新文件卡片、弹出菜单、徽标、对话框均复用已有 manager 类；无第二套主题或私有字体。跨窗口渲染观感在第 3 条待人验。

### 主题与对比度

3. [ ] **pending_human：明暗关键面截图**。本轮 E2E 是 DOM/状态断言，不是双主题成功截图对照；需浅/深文件列表、详情、打开方式、缺失、导入提示与预览稿并排核验。
4. [ ] **pending_human：完整渲染态对比度**。已重新计算 token 配色，见 design-audit.log：浅/深正文在 surface 为 11.22/13.76，secondary 7.70/8.31，muted 4.88/5.38；主按钮 11.22/4.57，hover 4.74/5.07；选中态主/secondary 均超过 4.5。不能把这视为任意 token 组合都通过：浅色 muted/sidebar=4.24、muted/canvas=4.47，选中底叠加后的某些 muted 配对也低于 4.5，须核验实际渲染是否使用这些配对（禁用/placeholder 另计）。本轮不根据不存在的组合虚报新缺陷，也不宣称全矩阵 AA；目标界面实际配对需实机复核并记录。
5. [ ] **pending_human：主题切换与重启首帧**。ThemeProvider 经 settings 后端设置主题，CSS 两套色；现有 E2E 有主题探针但不证明无首帧白闪、每个模板状态无跳变，需重启与连续切换目视验收。

### 字体与圆角

6. [ ] **pending_human：字体真实加载**。静态通过：10 个 @font-face 资源全部存在、构建打包字体，Figtree 零引用。真实字体非回退（包括中文 Noto）仍须运行时字体检查，不能用文件存在替代实际加载。
7. [x] **圆角（本 spec 静态）**：新模板 CSS 全用 card=18、row=14、pill=999，窗口=26；颗粒 inherit。全 CSS 另有既有 `.quick-search-window mark` 3px（index.css:1034）未满足四档规则，明确记录为 SPEC-21/快搜基线例外，不算本次新增、也不要求本轮实现者越界改 QuickSearchWindow。KWin 实际窗口见第 27 条。
8. [x] **中文无斜体、衬线 ≥18px（本 spec 静态）**：新建错误 em 的 manager-field-error 强制 font-style:normal；模板拖入标题为 18px 衬线，其余新增文本用正文。@font-face 的 italic 声明不是中文元素使用斜体；不将声明误报。

### 动效与无障碍

9. [x] **动效动机/时限（静态）**：quick=120ms、popup=180ms、manager=220ms，分别用于选中/弹出/悬停反馈。is-spinning=1s 是扫描持续反馈循环，不是一次交互响应延迟；未新增列表上浮。
10. [x] **reduced-motion（静态）**：全局覆盖 duration=0.01ms，theme-transition/animate-pop-in 强制 none，新增 is-spinning 在 reduce 下 none。无新增 JS 动画绕开该守卫；运行时偏好切换可随实机项复核。
11. [ ] **pending_human：全键盘焦点路径**。CSS focus-visible/within 为 2px outline + 2px offset；菜单有 role/aria、初始焦点与 Esc，对话框有初始焦点；隔离 E2E 有基础键盘回归。但不证明全 Tab/Shift+Tab、关闭恢复焦点、打开方式菜单和不同输入法路径全程可用，需人工走完；不冒称 E2E 覆盖所有辅助技术。

### 状态完整性

12. [ ] **fail：loading/empty/error**。扫描中、文件筛选/列表空态、缺失恢复指引、带错误码失败反馈有实现，E2E 覆盖主路径；**导入中缺状态，见 D1**。修复后应补延迟导入及失败清理状态断言；全部错误态的双主题外观仍随第 3 条待验。
13. [x] **缺失/自动导入/导出路径状态（范围化）**：FileTemplateDetail 有“文件缺失”及放回后重扫指引、自动导入标记；E2E 验证导入标记及保存清除，Rust 验证缺失/恢复。导出按钮按 SPEC-22 禁用并提示后续版本；**实际导出路径提示属 SPEC-23，本 spec 不适用**，不是遗漏实现。
14. [ ] **pending_human：信息密度 6**。静态仍是三栏、紧凑列表/文件信息卡片，无内容预览；需第 3 条实际截图检查中文截断、换行、菜单遮挡与视窗密度，不能从 DOM 数据断言推导审美通过。

### 隐私与安全（视觉侧）

15. [x] **敏感正文遮挡（静态）**：ManagerWindow:787 起为 isSensitive && !sensitiveRevealed 条件分支；遮挡时不挂载 CodeEditor。SPEC-22 没有改为将文件内容作为敏感文本编辑。真实会话/辅助技术的视觉体验仍可人工回归。
16. [x] **文件无敏感开关/不预览**：FileTemplateDetail 仅元数据、文件信息与动作，无内容加载或渲染；本轮 E2E 检查详情禁用动作和无敏感输入。
17. [ ] **fail：日志/提示隐私**。模板生产模块未见打印名称/路径，唯一 eprintln 为测试性能；但新建、改名和部分导入失败 toast 拼接文件名/标题，见 D2。文件卡片按需求显示名称本身不是失败。

### 回归

18. [ ] **pending_human：完整快捷键/IME 矩阵**。本轮 E2E 的快搜视觉/键盘与编辑历史回归通过，但不等于所有 Enter/Ctrl+Enter/Shift+Enter/Ctrl+O/Ctrl+E/Ctrl+数字/Esc 与真实 IME 全组合验收。文件快搜按键属 SPEC-23；全局快捷键/自动粘贴明确只留人验，未自动操作。
19. [x] **后端设置真相源（静态/既有 E2E）**：ThemeProvider 从 settingsApi.get/patch 同步，App 的 open-with 由模板后端命令记忆；没有新增以 localStorage 替代后端的业务状态。完整设置视图/备份属 SPEC-24。

### Searchis 专项

20. [x] **去玻璃**：src/ 中 backdrop-filter、surface-glass、canvas-glow 零匹配；未新增玻璃表面。拖入虚线 overlay 的 color-mix 是临时状态覆盖，不是常驻玻璃面板。
21. [x] **token 文档逐项静态对照**：design-audit.log 输出两主题全部 color/font/radius/motion/grain 值；与 design-tokens.md 逐项复核，显式列出的画布、surface、侧/列表/输入/代码/底栏、三级文字/disabled、边框、强调、主/次按钮、选中、图标底、focus/highlight、语义色、文件/语法色、四圆角、三字体一致（rgba 空格视为同值）。文档省略的阴影 alpha 序列不是精确数值合同；grain=0.06/0.035 尚待人工调定，不伪称最终实机值。
22. [x] **组件引用清单**：审计 16 个有运行时导出的组件模块，每个至少一条静态 import（见 design-audit.log）；HeaderBar 零引用。AboutModal 当前仍被 App 使用，其删除属于 SPEC-24，不能在此把它误当已删除死代码。模块引用不冒称每个命名导出均已 tree-shaking 分析。
23. [x] **颗粒不挡输入（静态）**：main-window-shell/quick-search-window ::after 明确 pointer-events:none；drop overlay 也为 none。真实 KDE 拖放事件传递仍属于 AC-24 实机回归，未用代码检查冒充 Dolphin 拖入实测。
24. [x] **Q-A 基线（自动化/范围化）**：快搜既有 Q-A 视觉/键盘 E2E 本轮通过；文本行维持类型图标、标题、Key/分组/次数、语言徽标、Ctrl 序号、高亮代码路径。文件行图标/徽标/按键由 SPEC-23 接续，不以这部分过渡态阻断 SPEC-22。
25. [x] **徽标规则（静态/自动化）**：管理文件行与详情用 ext.toUpperCase，FileTypeIcon 按文档/表格/pdf/代码分类；文本 language≠plain 才显示语言，SPEC-21 E2E 回归通过。快搜文件行延期范围同第 24 条。
26. [x] **最多一个主按钮（本 spec 静态）**：文件详情“打开”为主，其余（含保存）secondary；NewTemplateDialog 只有“创建”为主，PlusMenu/打开方式菜单无主按钮；打开创建对话框遮挡背景，非新增并列主动作。全屏各态视觉层级随第 3 条确认。
27. [ ] **pending_human：KWin 圆角/Global Menu**。隔离 KWin 中 E2E 通过不证明用户真实合成器下 26px 外轮廓和 KDE 五组 Global Menu 正常；必须由人操作真实桌面验证。

## Pending Human / Non-blocking Boundaries

- AC-26 Dolphin 改名移动（自动化只是代理）、AC-27 WPS 原子保存、真实打开方式 WPS/LibreOffice；不启动外部应用替代人工。
- 设计第 3/4/5/6/11/14/18/27 条的人验内容，以及真实拖入；双主题截图、字体实际命中、目标机性能与纹理最终值均未伪记已验。
- 已接受的目录级 TOCTOU 不重新升级；跨设备服务级端到端补偿并未被 helper 复现完全证明。
- 基线的快搜高亮 3px 圆角记录但不扩展本轮修复范围；快搜文件动作/导出提示归 SPEC-23，设置/About/zip 归 SPEC-24。

## Next Step

运行时可将 **R1、D1、D2** 退回 Implementer；Reviewer 不自行更改状态。R1 修复必须隔离源输入语义与根内移动语义，并保留 S2 回归。重新复审时运行针对性测试、cargo test、build 和 `.specflow/bin/e2e-isolated.sh`；继续明确列出 pending_human，不将全绿测试等同于全部设计/实机验收通过。
