# Searchis Master Plan

> 唯一需求源：`docs/prds/prd.md`（v0.4，2026-10-15 增补文件模板、分组与视觉定版）  
> 执行规则：`.agent/AGENT.md`  
> 技术基线：Tauri 2 + React 19/TypeScript/Vite + Rust + SQLite（SQLCipher 加密实现）

## 项目摘要

Searchis 面向 Arch Linux、KDE Plasma 6、X11，是单机、单用户的文本片段与文件模板检索、复制、自动粘贴与导出工具。V1 不包含账户、云同步、剪贴板历史、内置富文本/Markdown 编辑与渲染、音效、安装包或自动更新。

## 当前阶段

- 阶段名称：Phase 10 — 文件模板、分组与视觉定版（PRD v0.4）
- 阶段目标：废弃液态玻璃，落地浅色“植物”/深色“暖炭”主题；新增分组树与文本片段语言标记；新增文件模板（模板目录、导入、外部打开、目录扫描）；快搜窗口带格式复制、导出、打开；设置视图改版与 zip 备份。需求源：`docs/prds/prd.md` v0.4、`docs/prds/file-templates-ui-requirements.md`、`docs/research/xlsx-clipboard-spike.md`。
- 当前活跃 Spec：SPEC-20 视觉定版：植物 / 暖炭主题地基与 HeaderBar 迁移
- 当前状态：`in_progress`
- 当前责任角色：`Coder`
- 工作分支：`feat/file-templates`
- Last Updated：2026-10-15 Orchestrator 审阅通过 PRD v0.4 与 SPEC-20～24，SPEC-20 交 Coder

> Phase 9（液态玻璃，SPEC-17～19）已于 2026-08-28 完成，其玻璃配方将由 SPEC-20 回退。

## 技术决策

| 决策 | 结论 | 约束 |
|---|---|---|
| 桌面运行时 | Tauri 2 | 前端沿用仓库根目录 `src/` 的 React 19、TypeScript、Vite；系统能力只由 Rust/平台适配层调用 |
| 应用核心 | Rust | 承担应用服务、事务、检索、IPC 参数校验和错误映射 |
| 数据库 | SQLite Schema v1 + SQLCipher | “SQLite”不得退化为未加密 upstream SQLite；SQLCipher 负责数据库自身加密 |
| 密钥 | 首次启动生成独立随机密钥文件，权限 `0600` | 不进入源码、日志、数据库、可执行文件或 JSON 备份；密钥异常时禁止创建/打开明文库 |
| 进程/层次 | React UI → Tauri IPC → Rust 应用层 → 存储/检索/平台适配层 | UI 不拼 SQL，不直接执行平台命令，不维护第二份领域真相 |
| 发布目标 | Arch Linux + KDE Plasma 6 + X11 | Wayland、其他桌面/发行版/操作系统不在 V1 范围 |

## Vertical Slice 路线图

> 每个 spec 都必须产生用户可观察结果，并独立经过 Coder → QA。任一时刻只允许一个活跃 spec。

| 顺序 | Spec | 可观察交付 | 主要依赖 | 状态 |
|---:|---|---|---|---|
| 01 | [安全持久化首条片段](specs/01-secure-persistent-snippet.md) | 启动桌面应用、创建片段、重启后读取；数据库为 SQLCipher 加密 | 无 | `done` |
| 02 | [检索窗口与可靠复制](specs/02-search-and-copy.md) | 打开检索窗口、稳定检索、键盘选择、复制；失败不计使用次数 | 01 | `done` |
| 11 | [重建前端接入与双窗口形态](specs/11-rebuild-integration-dual-window.md) | 重建 UI（astryx/独立窗口组件）接真实后端；独立无边框检索窗口 + 单独管理窗口 | 01、02 | `done` |
| 03 | [KGlobalAccel 呼出与 X11 自动粘贴](specs/03-global-shortcut-autopaste.md) | `Alt+O` 呼出并向原窗口粘贴；能力不可用时仅复制 | 02、11 | `done` |
| 04 | [管理视图与敏感信息保护](specs/04-management-and-privacy.md) | 编辑、筛选、排序、置顶、标签、敏感正文会话级显隐 | 01、02 | `done` |
| 05 | [回收站完整生命周期](specs/05-trash-lifecycle.md) | 软删除、还原、永久删除、手动/自动清理及事务回滚 | 04 | `done` |
| 06 | [设置、主题与开机启动](specs/06-settings-theme-autostart.md) | 设置即时持久化，主题生效，XDG Autostart 真实状态回填 | 02、03、05 | `done` |
| 07 | [明文备份、恢复与示例重置](specs/07-backup-restore-reset.md) | 导出、校验预览、冲突处理、单事务导入与重置 | 05、06 | `done` |
| 08 | [六步首次使用向导](specs/08-onboarding-environment.md) | 首次启动完成环境检测、首条片段与检索演练，可续接/跳过 | 02、03、06 | `done` |
| 09 | [KDE Plasma 全局菜单](specs/09-kde-global-menu.md) | Global Menu 展示五组菜单，复用业务命令并在重启后重注册 | 03、04、07 | `done` |
| 10 | [发布候选与质量门槛](specs/10-release-quality-gate.md) | 可执行文件、校验值、依赖说明、性能/隐私/无障碍验收证据 | 01–09 | `done` |
| 12 | [快速检索窗口视觉重构并入](specs/12-quick-search-visual-refactor.md) | shadcn/cmdk 新视觉进入独立检索窗口，真实检索/粘贴/键盘行为不变 | V1 基线冻结 | `done` |
| 13 | [管理窗口与设置视觉重构并入](specs/13-manager-settings-visual-refactor.md) | 新版管理/设置视觉，真实 CRUD/设置/回收站契约不变 | 12 | `done` |
| 14 | [向导、快捷键帮助与 UI 清理回归](specs/14-onboarding-help-ui-cleanup.md) | 删除前端使用引导、重构快捷键帮助、清理 astryx，完成发布级回归 | 13 | `done` |
| 15 | [回收站操作与检索索引一致性](specs/15-searchindex-trash-consistency.md) | 软删除/还原/永久删除/清空/自动清理后检索结果与回收站状态一致 | — | `done` |
| 16 | [KDE AppMenu 实机显示恢复](specs/16-kde-appmenu-restore.md) | 五组菜单在 KDE Global Menu 实机显示、命令可触发、重启后 5s 内重注册 | — | `done` |
| 17 | [液态玻璃：设计 token 地基与快搜窗口玻璃](specs/17-liquid-glass-quick-search.md) | 快搜窗口玻璃面板 + 统一 token 体系（accent 电光蓝/radius 收敛/--quick-* 消灭）；检索复制粘贴不变 | 16 | `done` |
| 18 | [液态玻璃：管理窗口玻璃](specs/18-manager-window-glass.md) | 管理窗口侧栏玻璃 + 模态对话框玻璃 + 列表/详情实色 + 预存变量清理；Header 玻璃项作废（SPEC-13 重构已删 HeaderBar） | 17 | `done` |
| 19 | [液态玻璃：主窗口全量玻璃与明暗全量回归](specs/19-main-window-glass-full-regression.md) | 主窗口 shell 玻璃（真实透明 + 氛围光）+ 全量明暗回归（所有视图明暗两套） | 18 | `done` |
| 20 | [视觉定版：植物 / 暖炭主题地基与 HeaderBar 迁移](specs/20-botanical-ember-visual-foundation.md) | 去玻璃、两套不透明主题 token、字体随包、纸张颗粒、HeaderBar 按钮移到左栏底部 | 19 | `in_progress` |
| 21 | [分组树与文本片段语言标记（Schema v4）](specs/21-groups-and-language.md) | 最多 3 层分组树、语言下拉与 CodeMirror 高亮、类型筛选、快搜结果行 Q-A | 20 | `todo` |
| 22 | [文件模板管理（Schema v5）](specs/22-file-template-management.md) | 模板目录与分组同步、导入/新建/打开/打开方式/重命名/移动/回收站、目录扫描 | 21 | `todo` |
| 23 | [快搜窗口文件动作：带格式复制、导出与打开](specs/23-formatted-copy-and-export.md) | X11 选区持有、md/xlsx/文件带格式复制、Shift+Enter 导出、Ctrl+O、底栏切换、autoPaste 修正 | 22 | `todo` |
| 24 | [设置视图改版与 zip 备份](specs/24-settings-view-and-zip-backup.md) | 设置视图 S1、文件与导出分节、关于并入、zip 备份往返、重置清理模板 | 23 | `todo` |

## 看板

### Todo（Phase 10，按顺序串行）

- [ ] SPEC-21 分组树与语言标记
- [ ] SPEC-22 文件模板管理
- [ ] SPEC-23 带格式复制、导出与打开
- [ ] SPEC-24 设置视图与 zip 备份

### In Progress

- [ ] SPEC-20 视觉定版地基（Coder，2026-10-15 起）

- [ ] 无（SPEC-19 已 done，Phase 9 完成）

### QA Queue

- [x] SPEC-19 主窗口全量玻璃与明暗全量回归：QA `conditional_pass`（2026-08-28，build passed + 3 项机械 grep 全过 + 10 张明暗截图核验非空/区分度达标 + 范围核对干净）→ **Orchestrator 2026-08-28 目标机实机 Gate 通过，收为 `done`**
- [ ] SPEC-16 KDE AppMenu：QA `conditional_pass`（zbus DBusMenu 重做通过），待 Orchestrator 实机 Gate
- [x] SPEC-18 管理窗口玻璃：QA `passed`（2026-08-25） + Orchestrator 2026-08-26 决策（Header 项作废）收为 `done`

### Returned To Coder

- [ ] 无

### Returned To Coder

- [ ] 无

### Blocked

- [ ] 无（SPEC-14 已 done）

### 后处理项（Post-processing，不阻塞当前 SPEC，置后单独拆 slice 修复/验收）

- [ ] KDE AppMenu 实机未显示：已拆为 `specs/16-kde-appmenu-restore.md`，SPEC-15 后处理。
- [ ] 回收站检索索引失效：已拆为 `specs/15-searchindex-trash-consistency.md`，当前活跃。

### Gate 0 验证记录

- `cargo test --manifest-path src-tauri/Cargo.toml`：55 passed。
- `npm run build`：passed（1621 modules）。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`：passed。
- `npm run tauri:build`：passed；产物 `src-tauri/target/release/searchis`。
- `npm run test:e2e`：Gate 0 时未通过；SPEC-12 默认 external `tauri-driver`，并通过 custom Tauri service 在 session 已被 WDIO 删除时跳过 upstream 二次清理；默认 external 与 `E2E_DRIVER_PROVIDER=embedded` 复跑均 1 passed，无窗口标题/stale/undefined elementId 或 mock-store 清理警告。真实剪贴板/前台自动粘贴和目标机视觉仍须人工验收。

### Done

- [x] PRD 已建立：`docs/prds/prd.md` v0.3。
- [x] UI 交互基线已并入仓库根目录：`src/` 与 `src-tauri/`（原 `front-baseline/`）。
- [x] 技术栈确定为 Tauri、Rust、SQLite；为满足数据库自身加密约束，SQLite 采用 SQLCipher。
- [x] PRD 已拆分为 10 个可独立实现和验收的 vertical slices。
- [x] SPEC-01 安全持久化首条片段（18 tests, MT1+AC-02 passed, QA 验收通过 2026-08-03）
- [x] 已确认桌面端发布构建命令：`npm run tauri:build`（在仓库根目录执行）。
- [x] SPEC-02 检索窗口与可靠复制（32 tests + 性能 p95=7ms + 实机 MT1/MT2 全绿，QA 验收通过 2026-08-07）
- [x] SPEC-01/02 全部实现已提交（commit `96d5ead`）；重建前端（`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline`）已文件级覆盖到仓库根目录 `src/`（含 astryx 组件库与独立窗口组件），`npm run build` 验证通过；Rust 侧保留 SPEC-01/02 已验收成果未覆盖。
- [x] SPEC-11 重建前端接入与双窗口形态（55 tests + clippy + build，QA 验收通过 2026-08-08）
- [x] SPEC-03 KGlobalAccel 呼出与 X11 自动粘贴（55 tests + clippy + build，AC-03/04 人工通过，NFR-PERF 达标，RF1/RF2/RF3 已修复，QA 验收通过 2026-08-10）
- [x] SPEC-04 管理视图与敏感信息保护（55 tests + clippy + build，10 管理视图单元测试，AC-08/E-09/E-15 合规，QA 验收通过 2026-08-10）
- [x] SPEC-05 回收站完整生命周期（55 tests + clippy + build，5 回收站单元测试，AC-06/07/18/19 合规，事务回滚保证，QA 验收通过 2026-08-10）
- [x] SPEC-06 设置、主题与开机启动（55 tests + clippy + build，3 settings 单元测试，autostart 原子写入，三主题切换，AC-15 合规，QA 验收通过 2026-08-10）
- [x] SPEC-07 明文备份、恢复与示例重置（55 tests + clippy + build，后端完整实现，单事务导入/重置，AC-09/10 合规，QA 验收通过 2026-08-10）
- [x] SPEC-08 六步首次使用向导（55 tests + clippy + build，后端完整实现，环境检测/进度/完成，QA 验收通过 2026-08-10）
- [x] SPEC-09 KDE Plasma 全局菜单（55 tests + clippy + build，AppMenu D-Bus 适配，五组菜单模型，AC-16/17 合规，QA 验收通过 2026-08-10）
- [x] SPEC-10 发布候选与质量门槛（55 tests + clippy + build，AC-01~20 回归全通过，性能/安全/加密验证，V1 发布就绪，QA 验收通过 2026-08-10）
- [x] SPEC-13 管理窗口与设置视觉重构并入（RF5~RF7 复审 + RF8 release 实机通过；QA conditional_pass，Orchestrator 2026-08-21 放行转 `done`）
- [x] SPEC-15 回收站与检索索引一致性（SearchIndex::remove + 回收站五条写路径定向同步；62 tests，QA passed 2026-08-21；修复永久删除后仍可搜到）
- [x] SPEC-16 KDE AppMenu 实机显示恢复（zbus 导出 com.canonical.dbusmenu + RegisterWindow 真实 XID；70 tests，QA conditional_pass + Orchestrator 实测通过，2026-08-21 `done`；修复全局菜单不显示）
- [x] SPEC-17 液态玻璃：设计 token 地基与快搜窗口玻璃（统一 token 体系 + Figtree 随包发货 + 快搜窗口玻璃面板；QA passed + Orchestrator 2026-08-26 实机门禁通过，`done`）
- [x] SPEC-18 液态玻璃：管理窗口玻璃（侧栏玻璃 + 模态对话框玻璃 + 列表/详情实色 + canvas-glow F2 清理；`.manager-header` 玻璃项因 HeaderBar 已在 SPEC-13 重构时决定删除而作废，CSS 保留为未来锚点；QA passed 2026-08-25 + Orchestrator 2026-08-26 决策收为 `done`；实机截图 `artifacts/spec-18/01-manager-dark.png`）
- [x] SPEC-19 液态玻璃：主窗口全量玻璃与明暗全量回归（main 窗口 `transparent: true` + `.main-window-shell` 玻璃配方 + 氛围光伪元素 + 明暗全量回归 10 张截图；QA conditional_pass 2026-08-28 + Orchestrator 目标机实机 Gate 通过收为 `done`；**Phase 9 液态玻璃视觉定版全部完成**；commit `8ad133a`（prep）+ `25c0a8e`（SPEC-19））
- [x] Phase 8 后发布打磨：commit `617df2e`（2026-08-21，恢复 KDE Global Menu、系统托盘、sxhkd IPC、更新应用图标）与 commit `25e246e`（2026-08-24，快搜窗口初始激活时拉取默认 snippet 结果）已落地，两者均未绑定 spec（阶段收口性质）。
- [x] Gate 0：SPEC-03~10 状态与 QA 结论统一；Rust tests/frontend build/clippy/release build 通过；当前 V1 工作树冻结为 git 基线并标记 `v1-gate0-baseline`。E2E 多窗口 Driver 问题已作为明确限制记录。

## 跨 Spec 不变量

1. 所有 SQL 参数化；写操作在明确事务中执行。
2. `normalizedKey` 全表唯一，包含软删除记录；并发编辑以 `revision` 拒绝静默覆盖。
3. 日志、通知、错误、诊断和无障碍标签不得包含正文、查询词或剪贴板正文。
4. 自动粘贴只能发送固定 `Ctrl+V`；失败退化为仅复制，不得假报成功。
5. 敏感正文默认遮挡；数据库密钥绝不进入 JSON 备份。
6. UI 不直接访问 SQLite、D-Bus、X11、KGlobalAccel、剪贴板或 XDG Autostart。
7. 每个 spec 的 Coder 只修改其“允许修改范围”；跨范围需求退回 Master 拆解。
8. （v0.4）模板目录操作固定“先文件、后数据库”，失败撤销；任何补偿路径不得删除用户文件；模板路径必须位于模板目录内，不跟随符号链接。
9. （v0.4）自动化测试不得写真实系统剪贴板、不得启动外部应用；剪贴板与 `xdg-open` 在 E2E 构建中使用记录型替身。

## 决策记录

- 2026-08-01：采用文件作为 Master、Coder、QA 的唯一状态源。
- 2026-08-01：确认 Tauri 2、Rust、SQLite 技术栈；SQLite 通过 SQLCipher 满足 PRD A-11/D-03，禁止明文降级。
- 2026-08-01：采用单活跃 spec、按用户可观察闭环推进；基础设施必须随所属 vertical slice 一并验收，不单列“纯脚手架”任务。
- 2026-08-01：前端位于仓库根目录 `src/`（原 `front-baseline/` 已并入根目录），逐 slice 替换 mock 数据和 macOS 语义；音效能力只删除、不迁移。
- 2026-08-03：确认 `conditional_pass` 仅用于 QA 阶段结论；Spec 仍保持 `in_qa`，直到人工验收完成后才可转为 `done`。
- 2026-08-12：人工桌面 Gate 的缺失证据不构成 Coder Required Fix。自动检查通过且无具体 finding 时，QA 应给 `conditional_pass`、冻结代码并转 Orchestrator；只有人工 Gate 报告具体失败后才能退回 Coder。
- 2026-08-07：SPEC-02 在目标 Arch/KDE/X11 实机完成 MT1（真实剪贴板）与 MT2（键盘交互）验收，QA 升级为 `passed`；同日确认发布构建必须走 `npm run tauri:build`（纯 `cargo build --release` 不嵌入前端资源，烟测需验证 UI 渲染而非仅进程存活）。
- 2026-08-08：确认 PRD 双窗口形态（独立无边框置顶检索窗口 + 单独管理窗口，rofi/krunner 式）为需求本意；SPEC-01/02 的单窗口视图切换为过渡形态，由 SPEC-11 落地正式形态。
- 2026-08-08：重建前端覆盖决策——只覆盖前端层（`src/**`、`index.html`、`package*.json`），`src-tauri/**` 保留已验收 Rust 成果；重建前端为 mock 形态（searchEngine/initialSnippets），接入与适配全部由 SPEC-11 的 Coder 承担。
- 2026-08-08（QA F1 根因）：确认 `cargo build --release`（直接 Cargo 构建）不走 Tauri CLI 的前端嵌入流水线（`beforeBuildCommand` 不执行），产出的二进制可能嵌入过时/空的 `dist/` 资源。唯一正确的发布构建命令为 `npm run tauri:build`（该命令依次执行 `npm run build` + `tauri build`，在 Cargo 编译前完成前端资源嵌入）。
- 前端重构并入决策：外部 commit `0b28f05c...` 仅作为视觉/组件源，禁止整目录覆盖。不得并入其 mock `initialSnippets/searchEngine`、localStorage 领域数据、Option/Cmd 键位、音效/restoreClipboard 配置、单窗口 tauri.conf 或旧 Rust 源码。
- 并入顺序固定为 SPEC-12 快速检索 → SPEC-13 管理/设置 → SPEC-14 向导/帮助/清理；每个 slice 独立 Coder/QA，失败只回滚当前 slice。
- 2026-08-21：Orchestrator 确认 RF8 release 实机通过，SPEC-13 由 `in_qa`/`conditional_pass` 收为 `done` 并放行；SPEC-14 置为当前活跃 spec 交 Coder。
- 2026-08-21：Orchestrator 决定将两项独立发布阻塞（KDE AppMenu 未显示；Rust `SearchIndex` 未同步回收站操作）登记为**后处理项**，不阻塞当前 SPEC-14，待其完成后单独拆 slice 修复/验收。
- 2026-08-21：SPEC-14 收为 `done`，Phase 7 完成。两项后处理项分别拆为 SPEC-15（回收站检索索引一致性，当前活跃）与 SPEC-16（KDE AppMenu 实机显示恢复），进入 Phase 8。
- 2026-08-21（Phase 8 收口）：SPEC-15 QA `passed` 收为 `done`（重编后删除问题实机确认解决）；SPEC-16 实机第一次仍不显示，判 `qa_failed` 退回重做真实 DBusMenu（旧实现只 RegisterWindow 传字符串名、未导出 com.canonical.dbusmenu）；重做后 QA conditional_pass + Orchestrator 目标机实测通过，`done`。两项发布阻塞后处理项全部解除，Phase 8 完成。
- 2026-08-24：Phase 9 筹备定案 —— 液态玻璃视觉落地（SPEC-17 起）。设计源为 `~/hermes/design/searchis/` 下设计宪章与 glass spec v0.1（注意：这些文档内引用的旧编号 SPEC-15/16/17/18 属设计文档体系，与本仓库 spec 编号无关，仅作视觉源引用）。practice worktree 分支 `design-practice`（052bd4a/75befe3）只取配方作参考实现，禁止整体并入（主分支已分化）。
- 2026-08-25：SPEC-17 审阅通过并入库（.agent/specs/17-liquid-glass-quick-search.md）。审阅中做了库内证据核查：`--quick-*` 声明 3 处（index.css）但组件引用 5 处（QuickSearchWindow.tsx 2、ui/command.tsx 4，其中 2 处内联复用）→ 迁移需覆盖声明 + 组件引用；`--color-surface-elevated` 有 12 处引用但全部在 index.css 内，无 TSX 引用 → 删除前需先替换 index.css 内引用；astryx/legacy（`--canvas`/`--fg`）已零引用；accent 浅色 hue 35（暖橙）与深色 hue 200（冷蓝）分裂实为变速迁。Orchestrator 决策（2026-08-25）：**UI 字体 = Figtree 随包发货**；**电光蓝取值：深色 `#57a7ff` / 浅色 `#2f6df6`**（`#3d7bfd` 因浅色对比度 3.85:1 < 4.5:1 不采用）。spec 状态由 `todo` 推进为 `in_progress` 交 Coder。
- 2026-10-15（Phase 10 筹备）：需求讨论结论 `docs/prds/file-templates-ui-requirements.md` 定稿，PRD 升级到 v0.4（A-03 修订、A-14～A-17、FR-GRP/TPL/FMT/EXP/VIS、AC-21～38、D-11～D-18）；AGENT.md 同步内容边界、容量、备份格式与文件安全规则。Orchestrator 对 11 个待定问题全部采纳建议：分组最多 3 层；外部子目录自动成为分组；自动 Key 保留中文；标题与文件名双向联动；外部删除进回收站标记“文件缺失”；重名加 ` (2)`；只含文本片段的分组也建目录；字体打包 Playfair Display + Source Sans 3 拉丁子集、中文退回 Noto CJK；快搜窗口保留透明窗口；颗粒不透明度实机调定；接受深浅两套强调色不同。xlsx 带格式复制只带格式与值（预研 `docs/research/xlsx-clipboard-spike.md`）。
- 2026-10-15（Master 补充决定，Orchestrator 2026-10-15 审阅通过）：① 模板目录为 `<应用数据目录>/templates/`（`~/.local/share/io.searchis.desktop/templates/`，与数据库同目录，替代讨论稿中的 `~/.local/share/searchis/`）；② 识别顺序改为路径 → inode → 哈希；③ 单文件 20 MB、合计 10,000 条；④ 删除非空分组时内容整体上移到父级；⑤ 带格式复制、粘贴、导出计入使用次数，打开不计入；⑥ 导出后快搜窗口保持打开并显示路径；⑦ `autoPaste=false` 时 Ctrl+Enter 退化为仅复制（修正当前该设置在快搜窗口中不生效的问题）；⑧ 拆分为 SPEC-20～24 串行执行，视觉地基先行；⑨ Schema 分两步：v4（分组、语言，SPEC-21）、v5（文件字段，SPEC-22）。
- 2026-08-26：SPEC-18 Gate 复盘检出 **HeaderBar 死代码**——`HeaderBar.tsx` 组件存在但全仓零 import / 零 JSX 引用（`grep -rn 'import.*HeaderBar\|<HeaderBar' src/` 仅匹配组件自身 export）。根因为 commit `f4168e7`（SPEC-13/14/15 收口）将 `AppShell`/astryx 替换为 HTML/CSS divs 时，旧的 `<AppShell nav={<HeaderBar .../>}>` 被删除但新结构未补回 HeaderBar，提交信息谎称"迁移 HeaderBar"实际未迁。Orchestrator 决策：**(1) Header 已在 SPEC-13 重构时决定删除，不复活**（功能由侧栏小齿轮 + SettingsModal 外观主题 grid + KDE Global Menu 兜底），SPEC-18 成功标准中 `.manager-header` 玻璃项作废，CSS 保留为未来锚点；(2) SPEC-18 收为 `done`，其余 10 项（侧栏玻璃 + 列表/详情实色 + `.raycast-window` 玻璃 + F2 清理 + reduced-transparency 回退 + build + 实机截图）均通过；(3) 教训记录到 `headerbar-dead-code` 记忆，后续 SPEC 机械验收 grep 须含"未引用组件清单"项防复发；(4) SPEC-19 不再补回 header（保持 SPEC-13 决策一致）。

## 风险记录

- **（Phase 10）文件系统与数据库非原子**：导入、重命名、移动、回收站、备份导入都跨两种存储。缓解：先文件后数据库 + 补偿 + 目录扫描兜底收敛；SPEC-22 要求扫描核心为纯函数并覆盖注入失败测试。
- **（Phase 10）X11 剪贴板自实现**：INCR、多格式、选区丢失处理容易出错，且不能在自动化测试中验证真实粘贴。缓解：格式转换与选区持有解耦，选区层用 `x11rb` 最小实现；AC-30～32 必须实机手动验收。
- **（Phase 10）WPS 兼容性**：WPS 升级可能改变对 `XML Spreadsheet` 的解析。缓解：保留 `text/html` 与 TSV 兜底；预研样本留作回归用例。
- **（Phase 10）原子保存改变 inode**：外部编辑器保存后 inode 变化。缓解：识别顺序为路径 → inode → 哈希（PRD FR-TPL-03）。
- **（Phase 10）SPEC-21 至 SPEC-23 之间的备份缺口**：SPEC-24 前导出的 v1 JSON 备份不含分组、语言与模板文件。缓解：Phase 10 期间在 `feat/file-templates` 分支开发，SPEC-24 完成前不发布。

- **SQLCipher 构建/分发**：目标机动态库或链接策略可能影响单可执行文件交付。SPEC-01 必须记录实际依赖和密钥错误测试，SPEC-10 再做干净环境验证。
- **KDE/X11 集成**：KGlobalAccel、AppMenu 和窗口激活受 Plasma/D-Bus 生命周期影响。相关 spec 必须提供适配层替身测试与实机验收。
- **前端基线偏差**：现有 macOS 键位、mock 状态和音效设计不得直接成为产品行为；在涉及 slice 中逐项移除并最终形成差异清单。
- **性能**：10,000 条、平均正文 2 KB 的 p95/p99 目标需固定数据集和目标机样本；不得用开发机单次耗时宣称达标。
- **覆盖回退风险**：外部目录含单窗口 tauri.conf、旧 Rust、mock/localStorage 业务逻辑；全量复制会回退 V1。缓解：只按 spec 白名单移植视觉文件。
- **依赖与键盘风险**：cmdk/radix 可能接管键盘事件；必须以现有 Esc/Enter/Ctrl/IME E2E 为验收真相。
- **源漂移风险**：外部工作树可能继续变化；本轮只认 commit `0b28f05c230a544266eb508d52a66c9bf390d029`。
- **E2E 基础设施风险**：SPEC-12 默认使用已安装的 external `tauri-driver`，并通过 `browser.tauri.execute(..., withExecuteOptions({ windowLabel }))` 按 Tauri label 定向操作；默认及显式 external 复跑均通过且未出现窗口标题/stale/undefined elementId 或 mock-store 清理警告。`E2E_DRIVER_PROVIDER=embedded` 仅保留诊断用途；目标 Arch/KDE/X11 实机仍需 QA 验证。

## 当前活跃 Spec 状态卡

- Spec：SPEC-20 视觉定版：植物 / 暖炭主题地基与 HeaderBar 迁移
- 状态：`in_progress`（Coder）
- 下一步：Coder 实现并回写实施记录，转 QA
- Last Updated：2026-10-15 Orchestrator 审阅通过，SPEC-20 交 Coder
