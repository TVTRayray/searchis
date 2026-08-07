# Searchis Master Plan

> 唯一需求源：`docs/prds/prd.md`（v0.3）  
> 执行规则：`.agent/AGENT.md`  
> 技术基线：Tauri 2 + React 19/TypeScript/Vite + Rust + SQLite（SQLCipher 加密实现）

## 项目摘要

Searchis 面向 Arch Linux、KDE Plasma 6、X11，是单机、单用户的纯文本片段检索、复制与自动粘贴工具。V1 不包含账户、云同步、剪贴板历史、Markdown、音效、安装包或自动更新。

## 当前阶段

- 阶段名称：Phase 1 — 安全持久化首个闭环
- 阶段目标：建立可运行的 Tauri 桌面壳、加密 SQLite 数据库，并让用户创建一条片段后在重启后仍可读取。
- 当前活跃 Spec：`.agent/specs/03-global-shortcut-autopaste.md`（待 Master 产线就绪后启动）
- 当前状态：`SPEC-02 done`（QA `passed`，2026-08-07 实机验收全绿）
- 当前责任角色：`Master`（准备 SPEC-03）
- QA Status：`passed`
- 下一步：SPEC-03 KGlobalAccel 呼出与 X11 自动粘贴（前置 SPEC-02 已完成）

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
| 03 | [KGlobalAccel 呼出与 X11 自动粘贴](specs/03-global-shortcut-autopaste.md) | `Alt+O` 呼出并向原窗口粘贴；能力不可用时仅复制 | 02 | `todo` |
| 04 | [管理视图与敏感信息保护](specs/04-management-and-privacy.md) | 编辑、筛选、排序、置顶、标签、敏感正文会话级显隐 | 01、02 | `todo` |
| 05 | [回收站完整生命周期](specs/05-trash-lifecycle.md) | 软删除、还原、永久删除、手动/自动清理及事务回滚 | 04 | `todo` |
| 06 | [设置、主题与开机启动](specs/06-settings-theme-autostart.md) | 设置即时持久化，主题生效，XDG Autostart 真实状态回填 | 02、03、05 | `todo` |
| 07 | [明文备份、恢复与示例重置](specs/07-backup-restore-reset.md) | 导出、校验预览、冲突处理、单事务导入与重置 | 05、06 | `todo` |
| 08 | [六步首次使用向导](specs/08-onboarding-environment.md) | 首次启动完成环境检测、首条片段与检索演练，可续接/跳过 | 02、03、06 | `todo` |
| 09 | [KDE Plasma 全局菜单](specs/09-kde-global-menu.md) | Global Menu 展示五组菜单，复用业务命令并在重启后重注册 | 03、04、07 | `todo` |
| 10 | [发布候选与质量门槛](specs/10-release-quality-gate.md) | 可执行文件、校验值、依赖说明、性能/隐私/无障碍验收证据 | 01–09 | `todo` |

## 看板

### In Progress

- [ ] 无（SPEC-03 待 Master 产线就绪）

### QA Queue

- [ ] 无

### Returned To Coder

- [ ] 无

### Blocked

- [ ] 无

### Done

- [x] PRD 已建立：`docs/prds/prd.md` v0.3。
- [x] UI 交互基线已并入仓库根目录：`src/` 与 `src-tauri/`（原 `front-baseline/`）。
- [x] 技术栈确定为 Tauri、Rust、SQLite；为满足数据库自身加密约束，SQLite 采用 SQLCipher。
- [x] PRD 已拆分为 10 个可独立实现和验收的 vertical slices。
- [x] SPEC-01 安全持久化首条片段（18 tests, MT1+AC-02 passed, QA 验收通过 2026-08-03）
- [x] 已确认桌面端发布构建命令：`npm run tauri:build`（在仓库根目录执行）。
- [x] SPEC-02 检索窗口与可靠复制（32 tests + 性能 p95=7ms + 实机 MT1/MT2 全绿，QA 验收通过 2026-08-07）

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

## 风险记录

- **SQLCipher 构建/分发**：目标机动态库或链接策略可能影响单可执行文件交付。SPEC-01 必须记录实际依赖和密钥错误测试，SPEC-10 再做干净环境验证。
- **KDE/X11 集成**：KGlobalAccel、AppMenu 和窗口激活受 Plasma/D-Bus 生命周期影响。相关 spec 必须提供适配层替身测试与实机验收。
- **前端基线偏差**：现有 macOS 键位、mock 状态和音效设计不得直接成为产品行为；在涉及 slice 中逐项移除并最终形成差异清单。
- **性能**：10,000 条、平均正文 2 KB 的 p95/p99 目标需固定数据集和目标机样本；不得用开发机单次耗时宣称达标。

## 当前活跃 Spec 状态卡

- Spec：`SPEC-02`
- 状态：`done`
- Next Owner：`Master`（启动 SPEC-03）
- QA Status：`passed`
- Blocking Issue：无
- Required Fixes：无
- Retest Required：`no`（实机 MT1/MT2 已全数执行）
- Last Updated：2026-08-07
