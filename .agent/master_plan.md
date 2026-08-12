# Searchis Master Plan

> 唯一需求源：`docs/prds/prd.md`（v0.3）  
> 执行规则：`.agent/AGENT.md`  
> 技术基线：Tauri 2 + React 19/TypeScript/Vite + Rust + SQLite（SQLCipher 加密实现）

## 项目摘要

Searchis 面向 Arch Linux、KDE Plasma 6、X11，是单机、单用户的纯文本片段检索、复制与自动粘贴工具。V1 不包含账户、云同步、剪贴板历史、Markdown、音效、安装包或自动更新。

## 当前阶段

- 阶段名称：Phase 7 — 重构前端定向并入
- 阶段目标：把外部 shadcn/ui 视觉重构并入正式应用，同时保留已验收的真实数据、双窗口、KGlobalAccel/X11、设置、向导和 KDE AppMenu 契约。
- 外部视觉源：`/home/ray/.herdr/worktrees/searchis/rebuild-front/front-baseline` commit `0b28f05c230a544266eb508d52a66c9bf390d029`
- 当前活跃 Spec：`.agent/specs/12-quick-search-visual-refactor.md`
- 当前状态：`todo`（Gate 0 complete）
- 当前责任角色：`Coder`
- QA Status：`not_run`
- 下一步：Coder 实现 SPEC-12；只按白名单移植快速检索视觉，不整目录覆盖

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
| 12 | [快速检索窗口视觉重构并入](specs/12-quick-search-visual-refactor.md) | shadcn/cmdk 新视觉进入独立检索窗口，真实检索/粘贴/键盘行为不变 | V1 基线冻结 | `todo`（active） |
| 13 | [管理窗口与设置视觉重构并入](specs/13-manager-settings-visual-refactor.md) | 新版管理/设置视觉，真实 CRUD/设置/回收站契约不变 | 12 | `todo` |
| 14 | [向导、快捷键帮助与 UI 清理回归](specs/14-onboarding-help-ui-cleanup.md) | 新版向导/帮助，清理 astryx，完成发布级回归 | 13 | `todo` |

## 看板

### In Progress

- [ ] 无（SPEC-12 待 Coder 接手）

### QA Queue

- [ ] 无

### Returned To Coder

- [ ] 无

### Blocked

- [ ] 无

### Gate 0 验证记录

- `cargo test --manifest-path src-tauri/Cargo.toml`：55 passed。
- `npm run build`：passed（1621 modules）。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`：passed。
- `npm run tauri:build`：passed；产物 `src-tauri/target/release/searchis`。
- `npm run test:e2e`：**not passed**。E2E debug build 成功，应用主窗口正常渲染（失败截图已检查）；`@wdio/tauri-service` 在 embedded/external provider 均无法稳定绑定多窗口会话，产生 stale/undefined elementId，未进入可靠业务断言。已安装 `tauri-driver 2.0.6` 后复测仍失败。该限制不得在后续报告中写成通过，SPEC-12~14 必须修复/替换测试会话策略后再以 E2E 作为验收门。

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
- [x] Gate 0：SPEC-03~10 状态与 QA 结论统一；Rust tests/frontend build/clippy/release build 通过；当前 V1 工作树冻结为 git 基线并标记 `v1-gate0-baseline`。E2E 多窗口 Driver 问题已作为明确限制记录。

## 跨 Spec 不变量

1. 所有 SQL 参数化；写操作在明确事务中执行。
2. `normalizedKey` 全表唯一，包含软删除记录；并发编辑以 `revision` 拒绝静默覆盖。
3. 日志、通知、错误、诊断和无障碍标签不得包含正文、查询词或剪贴板正文。
4. 自动粘贴只能发送固定 `Ctrl+V`；失败退化为仅复制，不得假报成功。
5. 敏感正文默认遮挡；数据库密钥绝不进入 JSON 备份。
6. UI 不直接访问 SQLite、D-Bus、X11、KGlobalAccel、剪贴板或 XDG Autostart。
7. 每个 spec 的 Coder 只修改其“允许修改范围”；跨范围需求退回 Master 拆解。

## 决策记录

- 2026-08-01：采用文件作为 Master、Coder、QA 的唯一状态源。
- 2026-08-01：确认 Tauri 2、Rust、SQLite 技术栈；SQLite 通过 SQLCipher 满足 PRD A-11/D-03，禁止明文降级。
- 2026-08-01：采用单活跃 spec、按用户可观察闭环推进；基础设施必须随所属 vertical slice 一并验收，不单列“纯脚手架”任务。
- 2026-08-01：前端位于仓库根目录 `src/`（原 `front-baseline/` 已并入根目录），逐 slice 替换 mock 数据和 macOS 语义；音效能力只删除、不迁移。
- 2026-08-03：确认 `conditional_pass` 仅用于 QA 阶段结论；Spec 仍保持 `in_qa`，直到人工验收完成后才可转为 `done`。
- 2026-08-07：SPEC-02 在目标 Arch/KDE/X11 实机完成 MT1（真实剪贴板）与 MT2（键盘交互）验收，QA 升级为 `passed`；同日确认发布构建必须走 `npm run tauri:build`（纯 `cargo build --release` 不嵌入前端资源，烟测需验证 UI 渲染而非仅进程存活）。
- 2026-08-08：确认 PRD 双窗口形态（独立无边框置顶检索窗口 + 单独管理窗口，rofi/krunner 式）为需求本意；SPEC-01/02 的单窗口视图切换为过渡形态，由 SPEC-11 落地正式形态。
- 2026-08-08：重建前端覆盖决策——只覆盖前端层（`src/**`、`index.html`、`package*.json`），`src-tauri/**` 保留已验收 Rust 成果；重建前端为 mock 形态（searchEngine/initialSnippets），接入与适配全部由 SPEC-11 的 Coder 承担。
- 2026-08-08（QA F1 根因）：确认 `cargo build --release`（直接 Cargo 构建）不走 Tauri CLI 的前端嵌入流水线（`beforeBuildCommand` 不执行），产出的二进制可能嵌入过时/空的 `dist/` 资源。唯一正确的发布构建命令为 `npm run tauri:build`（该命令依次执行 `npm run build` + `tauri build`，在 Cargo 编译前完成前端资源嵌入）。
- 前端重构并入决策：外部 commit `0b28f05c...` 仅作为视觉/组件源，禁止整目录覆盖。不得并入其 mock `initialSnippets/searchEngine`、localStorage 领域数据、Option/Cmd 键位、音效/restoreClipboard 配置、单窗口 tauri.conf 或旧 Rust 源码。
- 并入顺序固定为 SPEC-12 快速检索 → SPEC-13 管理/设置 → SPEC-14 向导/帮助/清理；每个 slice 独立 Coder/QA，失败只回滚当前 slice。

## 风险记录

- **SQLCipher 构建/分发**：目标机动态库或链接策略可能影响单可执行文件交付。SPEC-01 必须记录实际依赖和密钥错误测试，SPEC-10 再做干净环境验证。
- **KDE/X11 集成**：KGlobalAccel、AppMenu 和窗口激活受 Plasma/D-Bus 生命周期影响。相关 spec 必须提供适配层替身测试与实机验收。
- **前端基线偏差**：现有 macOS 键位、mock 状态和音效设计不得直接成为产品行为；在涉及 slice 中逐项移除并最终形成差异清单。
- **性能**：10,000 条、平均正文 2 KB 的 p95/p99 目标需固定数据集和目标机样本；不得用开发机单次耗时宣称达标。
- **覆盖回退风险**：外部目录含单窗口 tauri.conf、旧 Rust、mock/localStorage 业务逻辑；全量复制会回退 V1。缓解：只按 spec 白名单移植视觉文件。
- **依赖与键盘风险**：cmdk/radix 可能接管键盘事件；必须以现有 Esc/Enter/Ctrl/IME E2E 为验收真相。
- **源漂移风险**：外部工作树可能继续变化；本轮只认 commit `0b28f05c230a544266eb508d52a66c9bf390d029`。
- **E2E 基础设施风险**：当前 `@wdio/tauri-service` 无法稳定绑定 Tauri 多窗口，debug build 与窗口渲染正常但 native WebDriver 会话产生 stale/undefined elementId。后续 slice 不得把该失败当业务通过；需先修复测试会话或改用可靠等价方案。

## 当前活跃 Spec 状态卡

- Spec：`SPEC-12`
- 状态：`todo`
- Next Owner：`Coder`
- QA Status：`not_run`
- Blocking Issue：无
- Required Fixes：先解决或替换 native 多窗口 E2E 会话策略，再以 E2E 作为验收门
- Retest Required：`yes`（SPEC-12 完成后完整 UI 回归）
- Last Updated：Gate 0 完成
