# Tasks: phase10-file-templates

> 串行执行：visual-foundation → groups-language → file-templates → formatted-copy-export → settings-backup。
> 每个 spec 的允许修改路径由任务包 `allowed_paths` 给出（来自 contracts 的“允许修改范围”）；更细的“仅限”约束以 contracts 为准。
> Implementer 完成后勾选本 spec 条目；“实机”条目由人在归档审批前完成，Implementer/Reviewer 不勾选。

## visual-foundation: SPEC-20 视觉定版：植物/暖炭主题地基与 HeaderBar 迁移

- [x] 1.1 `src/index.css` 替换为植物/暖炭两套 token，新增 highlight、文件类型、语法高亮、形状、字体、grain token；删除全部玻璃 token 与氛围光
- [x] 1.2 随包 Playfair Display（500、600、500 italic）与 Source Sans 3（400、500、600）拉丁子集 woff2，记录来源与 OFL 许可证；删除 Figtree
- [x] 1.3 纸张颗粒层（`pointer-events:none`）、输入框/按钮/焦点光环/图标 strokeWidth 规范、动效时长与 reduced-motion
- [x] 1.4 删除 `HeaderBar.tsx`，三个按钮移到管理窗口左栏底部，删除标题旁重复设置按钮
- [x] 1.5 主题展示名改为“浅色 植物 / 深色 暖炭 / 跟随系统”，存储值不变；`main` 窗口 `transparent=false`
- [x] 1.6 更新 E2E 视觉探针为新主题不透明背景色；机械 grep 无玻璃/Figtree/HeaderBar 残留
- [x] 1.7 对比度复核（正文、muted、选中、主按钮文字），数值写入 result（深色主按钮经决策 A 改为 #B35838 后 4.57:1，见 artifacts/spec-20/contrast-report.md）
- [x] 1.8 `npm run build`、`npm run test:e2e` 通过；记录字体总大小（预期 < 400 KB）
- [x] 1.10 落实 2026-10-15 决策：深色 `--color-btn-primary-bg` 改为 `#B35838`，复核深色主按钮文字对比度 ≥ 4.5:1 并更新 `artifacts/spec-20/contrast-report.md`；重新运行 `npm run build`、`npm run test:e2e`
- [ ] 1.9 实机：浅/深各 6 张截图存 `artifacts/spec-20/`，与预览页对照；左栏按钮与 KDE Global Menu 可用

## groups-language: SPEC-21 分组树与文本片段语言标记（Schema v4）

- [x] 2.1 Schema v4 迁移（groups 表、group_id、language、kind）、`.v3.bak`、失败回滚与只读错误页
- [x] 2.2 分组 IPC：list/create/update/delete_preview/delete，名称校验、同级唯一、深度 ≤ 3、成环检测、删除上移与 `(2)` 后缀，单事务
- [x] 2.3 左栏结构：全部/置顶/最近使用、分组树（折叠、+、未分组、计数）、标签区折叠、回收站、底部按钮栏
- [x] 2.4 文本片段表单增加分组与语言下拉；CodeMirror 6 编辑器（按需语言包），IME/撤销/粘贴/Tab 正常，敏感未展示不挂载正文；记录打包体积增量
- [x] 2.5 中栏类型筛选（全部/文本/文件）与列表行（类型图标、标题、Key、语言徽标）
- [x] 2.6 快搜结果行 Q-A 与匹配高亮；SearchResultItem 新增 language、groupName、kind；排序不变
- [x] 2.7 单元测试：分组校验/深度/成环/删除上移与回滚、迁移、language 非法值、搜索结果字段
- [x] 2.8 E2E：2 层分组、移动片段、语言徽标、删除非空分组；快搜行显示徽标与分组名
- [x] 2.9 性能：10,000 条片段 + 100 个分组下 NFR-PERF-02 p95/p99
- [ ] 2.10 实机：AC-23；浅/深管理窗口截图（含分组树与 bash 高亮）

## file-templates: SPEC-22 文件模板管理（Schema v5）

- [x] 3.1 模板目录与 `.trash/`（0700），首次补建分组目录；分组写操作同步目录
- [x] 3.2 Schema v5 迁移与约束、`.v4.bak`
- [x] 3.3 导入（多文件、拖入、20 MB 与 10,000 条上限、Key 自动生成与“自动导入”标记）与新建空白 md/xlsx
- [x] 3.4 右栏文件模板视图、标题与文件名双向联动、`(2)` 后缀
- [x] 3.5 `platform/files.rs`：xdg-open、MIME、`.desktop` 枚举与 Exec 字段码替换；打开方式记忆
- [x] 3.6 扫描纯函数核心 + IO 层：路径/inode/哈希识别、缺失与恢复、新文件与新子目录、超 3 层跳过、忽略规则、符号链接、分批事务、计数提示、“重新扫描”
- [x] 3.7 回收站文件生命周期与 SearchIndex 一致性
- [x] 3.8 先文件后数据库的补偿与 `TEMPLATE_RECONCILE_PENDING`；路径校验；日志隐私
- [x] 3.9 检索服务与管理检索包含文件模板（key/aliases/title/tags），管理列表扩展名图标与徽标；文本复制/粘贴对文件模板返回 TEMPLATE_ACTION_UNSUPPORTED；快搜窗口部分移交 4.5（2026-10-16 人工决策 B：快搜窗口中文件模板的行图标、徽标、按键与底栏移交 SPEC-23）
- [x] 3.10 单元测试：文件名清洗、Key 生成、扫描计划全部分支、路径校验、补偿、迁移、`.desktop` 解析
- [x] 3.11 E2E（临时 XDG、template_open 替身）：导入 md/xlsx、改标题同步文件名、回收站位置、外部改名后重新扫描
- [x] 3.12 性能：2,000 文件无变化扫描 ≤ 2 s（NFR-PERF-07）
- [ ] 3.13 实机：AC-26（Dolphin 改名移动）、AC-27（WPS 保存）、“打开方式…”列出 WPS/LibreOffice

## formatted-copy-export: SPEC-23 快搜窗口文件动作：带格式复制、导出与打开

- [ ] 4.1 `platform/x11_clipboard.rs`：选区所有者线程、TARGETS/TIMESTAMP/INCR/SelectionClear
- [ ] 4.2 格式转换纯函数：md→HTML/plain（转义原始 HTML、链接白名单）；xlsx→XML Spreadsheet/HTML/TSV（50,000 单元格上限、公式缓存值）；其他文件→uri-list/gnome-copied-files/plain
- [ ] 4.3 `template_format_copy` 计数幂等与失败不计数；`template_export` 文件名、`(2)`、不覆盖改名、无残留
- [ ] 4.4 `execute_paste` autoPaste 后端判定修正
- [ ] 4.5 快搜结果行文件模板图标与大写扩展名徽标（自 3.9 移交）；快搜按键（Enter/Ctrl+Enter/Shift+Enter/Ctrl+O/Ctrl+数字）、底栏切换、IME 防护；快捷键速查按类型分组；管理窗口“导出”“带格式复制”按钮启用
- [ ] 4.6 单元测试：md、xlsx（含预研样本）、HTML/TSV、上限、导出命名与失败、autoPaste、operationId
- [ ] 4.7 E2E（剪贴板替身）：底栏切换、Enter 格式集合与计数、Shift+Enter 写临时导出目录且窗口保持、autoPaste=false 不注入
- [ ] 4.8 性能：1 MB md、5,000 单元格 xlsx 各 100 次 p95（NFR-PERF-06）
- [ ] 4.9 实机（写真实剪贴板，需人执行或授权）：AC-30 WPS 文字、AC-31 WPS 表格、AC-32 Dolphin、纯文本编辑器粘贴

## settings-backup: SPEC-24 设置视图改版（S1）与 zip 备份、恢复、重置

- [ ] 5.1 设置视图 S1（分节导航、返回片段、Esc），AboutModal 并入“关于”分节并删除，Global Menu“关于”定位分节
- [ ] 5.2 “文件与导出”：导出目录选择/恢复默认/校验、文件名规则、模板目录只读路径与打开、明文说明；tauri-plugin-dialog 与 capabilities
- [ ] 5.3 备份 v2 导出（结构、上限、敏感确认、临时文件改名）
- [ ] 5.4 备份导入（v2 zip 与 v1 JSON、路径/符号链接/多余文件/哈希/解压炸弹校验、临时目录、单事务、补偿、启动清理、冲突策略、导入期间暂停扫描）
- [ ] 5.5 重置示例数据清理模板（目录改名、成功删除、失败改回）
- [ ] 5.6 单元测试：往返、各类拒绝、各阶段注入失败回滚、重置恢复、exportDir 校验
- [ ] 5.7 E2E：设置分节导航与返回、改导出目录后导出到新目录、AC-36 往返
- [ ] 5.8 性能：500 MB 模板导出/导入耗时记录，界面不冻结并显示进度
- [ ] 5.9 实机：KDE 目录对话框、Dolphin 打开模板目录、浅/深设置视图截图

## Completion Notes

仅记录可复现证据的链接或路径；QA 结论写入 `reviews/`；实机结果写入 `test_report.md`。
