# Proposal: phase10-file-templates

## Intent

PRD v0.4（2026-10-15）把 Searchis 从“文本片段工具”扩展为“文本片段 + 文件模板”工具：用户希望把 md、xlsx 等文件交给 Searchis 统一管理、按 Key 检索，在快搜窗口一键带格式复制、导出或打开，编辑交给 WPS、Typora 等外部软件；条目增多后需要分组树归类，代码类片段需要语言高亮。同时废弃 Phase 9 的液态玻璃，视觉定版为浅色“植物”、深色“暖炭”两套不透明主题。

本 change 对应原 `.agent/master_plan.md` 的 Phase 10（SPEC-20～24）。规划内容已于 2026-10-15 由 Orchestrator 审阅通过；2026-10-15 迁移到 SpecFlow，需求与拆分不变，只改变工件格式与状态管理。

## Scope

- 包含（按依赖串行，详见 `tasks.md`）：
  1. `visual-foundation`（SPEC-20）：植物 / 暖炭主题地基、去玻璃、字体随包、HeaderBar 按钮移到左栏底部。
  2. `groups-language`（SPEC-21）：Schema v4、最多 3 层分组树、文本片段语言与 CodeMirror 高亮、类型筛选、快搜结果行 Q-A。
  3. `file-templates`（SPEC-22）：Schema v5、模板目录与分组同步、导入/新建/打开/打开方式/重命名/移动/回收站、目录扫描。
  4. `formatted-copy-export`（SPEC-23）：X11 选区持有、md/xlsx/其他文件带格式复制、Shift+Enter 导出、Ctrl+O、底栏切换、autoPaste 修正。
  5. `settings-backup`（SPEC-24）：设置视图 S1、文件与导出分节、关于并入、zip 备份往返、重置清理模板。
- 不包含：全文检索、文件内容预览、内置富文本/Markdown/表格编辑器、inotify 实时监听、xlsx 公式/图片/图表复制、备份加密、Ctrl+K 动作面板、KDE Wayland 及其他平台。

## Affected Domains

- System Spec domain（归档后生成）：`visual-foundation`、`groups-language`、`file-templates`、`formatted-copy-export`、`settings-backup`。本轮 Delta 全部为 ADDED：项目尚无 System Spec 基线（既有行为的事实源仍是 `docs/prds/prd.md`），本 change 归档即形成第一版基线；对既有行为的修改（去玻璃、HeaderBar、Enter 语义、autoPaste）在 Requirement 文本中写明，并以 PRD v0.4 为准。
- 代码/数据/外部系统影响：Rust 存储层 Schema v3 → v4 → v5；新增模板目录（应用数据目录下 `templates/`）；新增 X11 CLIPBOARD 选区持有；备份格式 v1 JSON → v2 zip；新增依赖 CodeMirror 6、umya-spreadsheet、pulldown-cmark、x11rb、zip、tauri-plugin-dialog。

## Design Impact

- 档位：design_system
- 影响面（窗口/组件/状态）：快搜窗口、管理窗口三栏（左栏结构、列表行、详情区）、设置视图（由弹窗改为视图）、快捷键速查、关于、全部确认对话框；全部颜色/形状/字体 token。
- 证据路径：docs/prds/file-templates-ui-theme-preview.html（定版预览，botanical 与 claude-dark 两段）；docs/prds/file-templates-ui-requirements.md 第二部分；docs/ref/botancial.md（改编参考）
- 宪章变更：新建 `docs/design/design-charter.md`、`design-tokens.md`、`visual-qa-checklist.md`（取代 Phase 9 液态玻璃设计源），需随本计划一并人工批准。

## Success Criteria

- [ ] 五个 spec 的 Delta 场景全部有证据，Reviewer 结论均为 `pass`。
- [ ] PRD AC-21～38 全部通过；其中实机项（见下方约定）在归档审批前由人完成并记录于 `test_report.md`。
- [ ] `cargo test`、`npm run build`、`npm run test:e2e` 在最终提交上通过。
- [ ] 归档后 `.specflow/system-specs/` 生成五个领域的基线。

## Open Questions and Decisions Needed

- [x] System Spec 基线：本轮全部写成 ADDED，归档即形成第一版基线（2026-10-15 采纳）。
- [x] 实机验收约定（2026-10-15 采纳）：Reviewer 只对可自动化证据下结论；无法自动化的项（KDE 全局快捷键、写真实剪贴板、自动粘贴、WPS/Dolphin 粘贴、实机截图对照、KDE Global Menu 显示）在 QA 报告中列为 `pending_human`，不阻止 `pass`；人在归档审批前统一执行并在 `test_report.md` 勾选，失败则 `specflow reopen-qa <change> --spec <spec> --reason ... --confirm` 退回该 spec。写真实剪贴板须由人执行或明确授权。
- [x] 深色主按钮对比度（visual-foundation 实现中发现，2026-10-15 人工决策 A）：深色 `--color-btn-primary-bg` 由 `#B95C3D`（4.28:1）改为 `#B35838`（4.57:1），悬停色 `#A9522F`（5.07:1）不变；已回写 `docs/design/design-tokens.md`、需求文档 token 表与预览页。浅色 placeholder 4.24:1 不在 MUST 列表内，交由 Reviewer 判断。
- [x] visual-foundation 修复轮越界（2026-10-15 人工决策 A）：授权保留 ThemeProvider 的 reduced-motion 守卫、command.tsx 焦点环包裹元素、e2e 本 spec 回归断言；contract 20 允许修改范围已同步放宽。
- [x] file-templates 规划缺口（2026-10-16 人工决策 B）：SPEC-22 路径未含 QuickSearchWindow.tsx；快搜窗口中文件模板的行图标、徽标、按键与底栏移交 formatted-copy-export（Delta、contracts 22/23、tasks 3.9/4.5 已同步）。过渡期快搜对文件模板按 Enter 返回 TEMPLATE_ACTION_UNSUPPORTED，分支未发布，可接受。
- [x] file-templates 非阻断项（2026-10-16 人工接受）：① 导入文件选择使用系统 kdialog/zenity（平台原生能力优先），settings-backup 引入 dialog 插件时可统一；② v1 JSON 备份导出跳过文件模板，完整备份由 settings-backup 的 zip 提供。
- [x] file-templates 安全修复（2026-10-16 人工决策）：S1 静态符号链接逃逸修复（决策 A）；残余目录级 TOCTOU 接受为安全例外（决策 B），威胁模型写入 contracts/22「安全与隐私」。
- [x] formatted-copy-export 规划缺口（2026-10-16 人工决策 A）：管理窗口“导出”“带格式复制”按钮在 SPEC-22 新增的 `ui/FileTemplateDetail.tsx` 中；经 `specflow add-path` 加入 SPEC-23 范围（仅按钮启用与接线），contracts/23 已同步。非阻断：Settings 新增只读 exportDir 前端类型（可选字段），修改界面由 settings-backup 提供。
- [x] SPEC-20 遗漏（2026-10-19 人工决策）：`index.html` 仍从 Google Fonts 远程加载 Figtree（启动访问外网，违反“Figtree 零引用、字体随包”），并入 settings-backup 修复轮处理；index.html 加入 SPEC-24 范围。
- [x] 实机验收发现（2026-10-19，人工授权直接修复、免二次 QA）：管理窗口文本内容编辑器（CodeMirror，SPEC-21 引入）继承 `.manager-control` 的胶囊圆角 999px，占位文字被遮挡；`.manager-code-editor` 改为 `--radius-card`（18px）与代码块底色、内描边，与原 textarea 一致。
- [x] 颗粒纹理不透明度（2026-10-19 人工验收）：沿用浅色 0.06、深色 0.035 作为最终值，已写回 `docs/design/design-tokens.md`。

## Source Requirements

- PRD / issue / discussion：`docs/prds/prd.md` v0.4（A-03、A-14～A-17、FR-GRP/TPL/FMT/EXP/VIS、AC-21～38、D-11～D-18）；`docs/prds/file-templates-ui-requirements.md`；`docs/research/xlsx-clipboard-spike.md`；`.agent/master_plan.md` 决策记录 2026-10-15 两条。
- 详细契约：`contracts/20～24-*.md`（原 `.agent/specs/`，2026-10-15 迁入本 change）。
- 项目规则：`.agent/AGENT.md`（项目宪法）。

## Input PRDs

- docs/prds/prd.md
- docs/prds/file-templates-ui-requirements.md
- docs/research/xlsx-clipboard-spike.md
