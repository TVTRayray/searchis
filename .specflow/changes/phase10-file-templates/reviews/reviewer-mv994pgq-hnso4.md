# QA Report: reviewer-mv994pgq-hnso4

## Verdict

`pass`。仅针对 visual-foundation / SPEC-20 的可自动化验收范围；不是整个 Phase 10 完成、实机验收通过或归档批准。由 runtime 导入结论；本 reviewer 不修改 state 或自行标记 qa_passed。

已阅读任务包、proposal、tasks、Delta Spec、contract 20、design、ux-design、项目宪法、设计宪章/tokens/视觉清单，审查实际工作区 diff、历史 QA 与三轮 implementer result。contract 20 的 command.tsx 包裹元素、ThemeProvider reduced-motion 守卫、E2E 本 spec 回归断言三处均已注明 2026-10-15 人工授权；上一轮 S1 已解决，不再作为阻塞。

## Spec Coverage

| Requirement / scenario | Evidence | Result |
|---|---|---|
| 不透明主题、无玻璃残留 | src 机械 grep 零结果；main transparent=false、search=true；实际表面使用不透明 canvas/surface tokens | 通过；KWin 实机 pending_human |
| 植物/暖炭 token、切换仅改颜色 | 本轮 Node 对照设计表全部 34 组具明确色值的颜色配对（含表内组合行）；共同形状/字体定义；E2E 源码及最终实现证据涵盖明暗/system | 通过；截图外观 pending_human |
| 深色重启首帧 | main.tsx 挂载前恢复缓存；ThemeProvider 后端纠正逻辑保留，未改变缓存/后端权威关系 | 源码回归通过；WebView 首帧/白闪 pending_human，不声称动态冷启动已验 |
| 字体真实加载、共同形状、纹理 | 六个新增 woff2 均进入本轮 build；OFL 文件；字体来源见首轮 result；SVG ::after 为 pointer-events:none；实现 E2E 字体加载探针 | 通过；CJK 缺失回退、纹理最终观感 pending_human |
| 控件、动效、reduced-motion | F1 容器焦点环、F2 CSS 同 specificity 后置覆盖和 ThemeProvider 守卫；三处 placeholder 使用 secondary；120/180/220ms tokens、无列表上浮 | F1–F3 已修复，详见下节 |
| HeaderBar 迁移 | HeaderBar 已删除且 src 零引用；App 将原 theme/settings/help 回调传入 ManagerWindow，底栏横排，标题旁重复设置按钮删除；E2E 检查三个按钮及后端主题持久化 | 通过；KDE Global Menu pending_human |
| 主题展示名、旧值有效 | THEME_LABELS 与设置文案；light/dark/system 未变；未改 API、持久化或迁移协议 | 通过；旧 dark 仍映射深色暖炭 |
| 对比度 | 本轮从 CSS token 独立按 WCAG 2.x 复算正文、muted、选中、主按钮、placeholder 的实际底色；与 contrast-report 一致 | 通过，数值见 Tests Reviewed |
| 边界/失败场景 | reduce 规则及 fallback 字体栈保留；无 Rust/API/data diff，无新增异常处理或业务路径 | 自动化范围通过；合成器关闭、系统字体缺失见 pending_human |

## Findings

无新增需退回的阻塞发现。上一轮问题逐项关闭：

- **S1 — permission_insufficient：resolved。** contract 20 的三处人工授权覆盖实际修复；未要求继续修改权限外代码。
- **F1 — accessibility：resolved。** `command.tsx` 只新增无行为 `.quick-search-field` 容器；`index.css` 用 `:focus-within` 绘制 `2px solid var(--color-focus-ring)`、`outline-offset:2px`，输入框仍无边框，容器为 field 填充底与胶囊形。原来锁定“无焦点环”的 E2E 已换为正向检查容器焦点、填充底、颜色、宽度、偏移；按设备像素取整宽度与实现者报告的 1.6px 一致，不是删除焦点指示。输入行 54px 保留；新胶囊观感待实机截图。
- **F2 — accessibility：resolved。** reduce 媒体规则内 `.theme-transition`、后代及伪元素与原规则同 specificity、源码后置，`transition:none !important` 能覆盖 220ms shorthand；ThemeProvider 同时避免在 reduce 下添加 class。E2E 强制挂 class 后检查计算时长，覆盖原缺陷发生期间，而不是仅检查 class 移除后。最终 implementer 报告本机 nativeReduce=true、时长为 0；临时删除覆盖的负向验证得到 220ms 并按预期失败，恢复后最终重跑通过。
- **F3 — accessibility：resolved。** 管理搜索、编辑 input/textarea、快搜三处 placeholder 均改为现有 secondary token，不修改已批准色值。浅/深 secondary 对 field 为 **6.6935/7.2480**，对 surface 为 **7.7008/8.3091**，均 ≥4.5。E2E 同时校验规则使用 secondary 与两套背景的比值。

非阻塞证据限制：E2E 本轮按用户要求仅审核实现者证据；reduce=false 环境下的注入媒体规则分支未有本轮实跑记录，不冒充完整操作系统偏好矩阵覆盖。实机待验不算自动化缺陷，按 proposal/宪法约定保留。

## Tests Reviewed

### 本轮实际执行

- `npm run build`：**pass**（tsc + Vite，1667 modules，1.95s）；六个新字体均在 dist 输出。
- `git diff --check`：**pass**。
- `grep -rnE 'backdrop-filter|glass|canvas-glow|Figtree|HeaderBar' src/`：零结果（预期 exit 1）。
- `grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components src/App.tsx`：零结果。
- `grep -rnE 'rounded-|borderRadius' src/components src/App.tsx`：零结果；组件通过 CSS 类使用共享圆角档位。
- `git diff HEAD --name-only -- src-tauri/src src/api src/types src/lib`：零结果。Rust scoped test **not_applicable**，未改 Rust 逻辑。
- Node token 对照：**pass**，34 组明暗颜色与批准表一致（包括深色主按钮 #B35838；rgba 空格及小数形式规范化比较）。其余形状/字体/阴影/纹理与动效源码人工对照。
- Node WCAG 复算（浅/深）：正文 surface **11.2210/13.7585**，列表底 **10.7469/14.2876**，侧栏底 **9.7413/15.1044**；muted surface **4.8821/5.3792**、列表底 **4.6758/5.5860**；选中态正文在 surface 上 **9.3994/10.9176**、列表上 **9.0587/11.3320**、侧栏上 **8.3319/11.9925**（先 alpha 合成）；主按钮 **11.2206/4.5680**、hover **4.7419/5.0728**；placeholder 数值见 F3。全部 ≥4.5。
- `wc -c src/assets/fonts/*.woff2`：全量 **203,136 B**；新增两字体共 **116,368 B**（Playfair 69,352 B、Source Sans 3 46,016 B），低于 400 KB。

### 转引并审核的实现者证据（不是本轮亲自运行）

- `handoffs/implementer-mv98o4zo-bexgr.result.yaml`：最终 `npm run build` pass；最终 `npm run test:e2e` **1 passing，6.3s**；F2 负向验证按预期失败，恢复后重跑通过。
- 已核对 `e2e/create-and-search.e2e.ts` 的最终断言：字体加载、底栏三个按钮、主题名称/后端持久化/跟随系统、管理和快搜不透明颜色、26px、颗粒、一个主按钮、F1/F2/F3。
- 前两轮 result 提供最初实现、字体来源和深色主按钮决策的连续证据；不沿用旧 F1 反向断言或旧 placeholder 例外结论。
- **本轮未执行 E2E、未启动桌面应用、未执行 xdotool 前台激活/真实输入、未写真实剪贴板或自动粘贴。**

## Design Conformance

逐条对应 `docs/design/visual-qa-checklist.md`。`[x]` 指本 spec 的静态检查/本轮检查或明确标注的实现者自动化证据足够；不表示实机已验。`[ ]` 明确标注待人工或非本 slice 范围。

### 一致性
- [x] 表面使用同一 token 体系；组件/App 颜色与圆角硬编码机械检查零结果。
- [x] 强调色、共享圆角和字体由统一 CSS 定义；明暗不另设布局。

### 主题与对比度
- [ ] 明暗关键面截图对照：pending_human，artifacts/spec-20 当前仅 contrast-report.md。
- [x] 正文、muted、选中、主按钮及 placeholder 实际背景配对达标，本轮独立复算。
- [ ] 无白闪/跳变及冷启动首帧：pending_human；主题即时切换/后端持久化有实现者 E2E，源码挂载前缓存路径保留。

### 字体与圆角
- [x] 随包文件、字体声明及本轮 build 一致；加载探针按实现者 E2E；Figtree 零引用。
- [x] 26/18/14/999 共享档位已定义并用于主体窗口、卡片/代码区、行与胶囊控件；物理窗口圆角 pending_human。
- [x] ≥18px 详情/设置分节/空状态等标题使用衬线；未新增中文斜体。小型对话框标题保留正文字体。

### 动效与无障碍
- [x] 常规反馈 quick 120ms、pop 180ms、manager 220ms，无列表上浮；非装饰性拖慢。
- [x] reduce 覆盖和主题切换守卫有效，F2 回归/负向验证见实现者证据。
- [x] 全局 focus-visible 与快搜容器焦点环符合规范（F1）；业务键盘处理未改，全量键盘/IME 实机回归仍待人工。

### 状态完整性
- [x] 本 slice 保留既有 loading/empty/error 路径；扫描/导入/分组/文件筛选状态属后续 spec，未提前实现。
- [ ] 文件缺失/自动导入/导出路径：not_applicable，SPEC-22～24。
- [x] 仍为三栏管理与紧凑快搜，未扩张业务结构；视觉密度最终截图确认 pending_human。

### 隐私与安全（视觉侧）
- [x] 敏感遮挡与正文挂载条件未改；本轮为真实 diff 静态回归，不声称独立动态安全测试。
- [ ] 文件模板敏感开关/内容预览：not_applicable，后续 spec。
- [x] 无新增日志、正文提示或平台调用；Rust/API/types/lib 无差异。

### 回归
- [x] 实际 diff 未改键盘/IME handler；实现者 E2E 保留选中、复制替身、退出等回归；不代替全部组合键实机验收。
- [x] 底栏主题回调仍更新后端设置；localStorage 只是原有缓存，存储协议不变。

### Searchis 专项
- [x] 玻璃、氛围光、HeaderBar 源码零引用；HeaderBar 文件已删。
- [x] 颜色 token 逐项对照，形状/字体/纹理人工源码核对。
- [x] 本 slice 触及的组件仍有使用；AboutModal 在当前 App 中仍挂载，删除属 SPEC-24，不误判为死代码。本次未做无关组件的仓库全量死代码认证。
- [x] SVG 纸张层 pointer-events:none；实现者有计算样式探针，最终不透明度调定 pending_human。
- [ ] Q-A 行类型/分组新信息：not_applicable，SPEC-21。
- [ ] 文件/语言徽标新规则：not_applicable，SPEC-21/22。
- [x] 编辑现有条目时快速粘贴主操作、保存描边；新建时保存为主；确认框只有确认主操作，E2E 管理视图主按钮数量探针通过。
- [ ] KWin 26px 与 KDE Global Menu 五组菜单：pending_human。

## 实机验收

以下须由人在归档前写入 test_report.md，按已批准约定不阻止本次 pass：

- pending_human：浅/深各六张截图（快搜、管理、设置、快捷键、关于、确认），与定版预览对照；包括新快搜填充胶囊观感。
- pending_human：左栏三个按钮设备交互与 KDE Global Menu 五组菜单。
- pending_human：KWin 26px、合成器关闭降级、颗粒强度最终调定与回写。
- pending_human：main 不透明窗口深色冷启动首帧、主题切换无白闪。
- pending_human：缺少 Noto CJK 的回退与完整键盘/IME 回归。

## Next Step

将本报告与 result 交 runtime 导入；reviewer 不自行推进状态。保留 tasks 1.9 未勾选，不批准整个 change 或归档。实机失败由 runtime 按流程 reopen-qa。
