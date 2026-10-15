# Searchis 项目执行宪法

> 适用范围：本仓库的 spec-first 开发流程（2026-10-15 起采用 SpecFlow，见 §8～§11）  
> 需求真理源：`docs/prds/prd.md`  
> UI 交互基线：仓库根目录 `src/`（原 `front-baseline/` 已并入仓库根目录）

## 1. 项目上下文

- 项目名称：Searchis
- 产品定位：Arch Linux 本机文本片段与文件模板的检索、复制、自动粘贴与导出工具。
- 唯一目标环境：Arch Linux、KDE Plasma 6、X11。
- 默认全局呼出快捷键：`Alt + O`，用户可修改。
- 数据边界：单机、单用户、本地存储；不建设账户、服务端、云同步或跨设备同步。
- 内容边界（PRD v0.4）：条目分为文本片段（UTF-8 纯文本，可带语言标记）与文件模板（任意文件，明文存放在 `<应用数据目录>/templates/`）。不内置富文本/Markdown/表格编辑器，不在界面中渲染 Markdown 或预览文件内容；Markdown 只在带格式复制时转换为剪贴板 HTML。
- 交付形式：只交付可执行文件，不交付 PKGBUILD、安装包或应用内自动更新。
- 当前前端基线：React 19、TypeScript、Vite，位于仓库根目录 `src/`；Tauri 壳位于 `src-tauri/`。
- 当前技术基线：Tauri 2 + React 19/TypeScript/Vite + Rust + SQLite；为满足数据库自身加密约束，SQLite 必须采用 SQLCipher 加密实现，不得退化为明文 upstream SQLite。

## 2. 产品硬约束

以下约束只能通过修改 `docs/prds/prd.md` 和对应 spec 后变更：

1. 文本片段与文件模板合计最多 10,000 条未删除条目；单条文本正文最大 100 KB，单个模板文件最大 20 MB。
2. 本地数据使用最终选定数据库自身的加密能力。
3. 数据库密钥由应用本地生成，保存到权限为 `0600` 的独立文件；禁止硬编码或写入日志、数据库、可执行文件和 JSON 备份。
4. 备份为明文 zip（`backup.json` + `templates/`，PRD v0.4；仍可导入旧版 JSON 备份）；包含敏感片段时必须二次确认。
5. 回收站支持手动清空及关闭、7 天、30 天、90 天自动清理；默认关闭。
6. 不实现任何操作音效、音效配置或音频资源。
7. 保留 6 步首次使用向导。
8. KDE Plasma 全局菜单采用 PRD `FR-KDE-01` 定义的五组菜单。
9. 不采集、读取、记录或索引其他应用产生的剪贴板历史。
10. 日志、通知、错误提示和无障碍标签不得包含片段正文、查询词或剪贴板正文。

## 3. 架构边界

### 3.1 当前决策状态

- 仓库根目录 `src/` 与 `src-tauri/` 是正式实现，同时承担交互参考；仍须按 PRD 修正 Linux/KDE 语义。
- 桌面运行时采用 Tauri 2，应用核心与平台适配采用 Rust，存储采用 SQLite Schema v1 + SQLCipher。
- 具体 Rust 绑定和系统集成库由所属 vertical slice 以目标 Arch 环境实测决定；不得改变 Tauri/Rust/加密 SQLite 技术边界。
- 依赖决策顺序：平台原生能力 → 已存在依赖 → 成熟依赖 → 最小本地实现。
- 不得搭建与产品无关的通用框架、插件系统、云端接口或跨平台兼容层。

### 3.2 分层规则

无论最终采用何种桌面运行时，都必须保持以下边界：

- UI 层：渲染视图、收集输入、展示状态；不得直接拼接 SQL 或自行维护第二份领域数据。
- 应用层：编排检索、复制、自动粘贴、CRUD、导入导出和回收站事务。
- 平台适配层：封装 KGlobalAccel、X11 窗口激活与按键注入、D-Bus AppMenu、XDG Autostart 和系统剪贴板。
- 存储层：负责 Schema、参数化查询、唯一约束、事务、迁移、加密和并发版本控制。
- 检索层：实现 PRD `FR-SCH-02` 的稳定排序；软删除记录不得进入正常索引。

UI 不得直接调用未封装的平台命令。平台能力失败必须转换为 PRD 定义的错误码，不得静默吞错。

## 4. 数据与安全规则

- 所有数据库查询必须参数化；禁止字符串拼接 SQL。
- 新增、编辑、删除、恢复、清理、导入和重置必须使用事务或等价原子机制。
- `normalizedKey` 必须建立包含软删除记录的唯一约束。
- 写入必须保留 `revision` 并拒绝静默覆盖并发修改。
- 禁止把片段正文解释为 HTML、脚本、命令或按键序列；禁止执行 bash/SQL 等文本片段。
- Markdown 转剪贴板 HTML 时必须转义原始 HTML（PRD FR-FMT-02），转换结果不在 UI 中渲染。
- 文件系统操作（模板目录）与数据库事务不是原子的：固定“先文件、后数据库”，数据库失败时撤销文件操作；任何补偿路径都不得删除用户文件。
- 模板目录内的路径一律经过规范化校验，拒绝绝对路径、`..` 与符号链接；打开文件、启动应用以参数数组调用，不经过 shell。
- 日志不得记录模板文件名、相对路径或文件内容。
- Web UI 禁止使用 `dangerouslySetInnerHTML` 渲染用户数据。
- 自动粘贴只允许发送固定 `Ctrl + V`。
- 数据库密钥不可恢复时不得退化为明文数据库。
- 导入文件必须先完成大小、编码、JSON 和 Schema 校验，再进入单一事务。
- 永久删除、清空回收站、重置数据和导出敏感片段必须执行 PRD 规定的确认流程。

## 5. KDE / X11 系统集成规则

- 全局快捷键通过 KDE KGlobalAccel 注册；注册失败时保留原有效快捷键。
- 自动粘贴前必须记录并验证呼出前的 X11 目标窗口。
- 带格式复制由应用自己持有 X11 CLIPBOARD 选区，必须响应 `TARGETS` 并支持 INCR 大数据传输；其他剪贴板写入继续走现有封装。
- X11 键盘事件注入不可用时退化为仅复制，不得报告自动粘贴成功。
- KDE 全局菜单通过 D-Bus AppMenu 导出；Plasma Shell 或 Registrar 重启后 5 秒内重新注册。
- Global Menu 小部件缺失不得阻塞应用启动或应用内操作。
- 开机启动使用 XDG Autostart `.desktop` 条目，并以系统真实状态回填 UI。
- KDE Wayland、其他桌面环境、其他发行版及其他操作系统不属于 V1 兼容范围。

## 6. UI / UX 规则

- 仓库根目录 `src/` 提供布局和交互参考；实现必须按 PRD 将 macOS 键位和文案改为 Linux/KDE 语义。
- 必须支持浅色、深色、跟随系统三种主题；v0.4 视觉为浅色“植物”、深色“暖炭”，全部表面不透明、不使用 `backdrop-filter`，规范以 `docs/prds/file-templates-ui-requirements.md` 第二部分第 6 节为准。
- 使用语义化主题变量，禁止在新增业务组件中散布无语义硬编码颜色。
- 所有核心动作必须可通过键盘完成，并正确处理输入法组合态。
- 敏感正文默认遮挡，且不得进入默认可见文本或无障碍标签。
- 不得新增操作音效、音效开关、音频依赖或音频资源。
- 所有错误必须说明失败对象、原因类别和用户可执行的下一步。

## 7. 代码质量与验证

### 7.1 通用门槛

- 行为变更必须关联一个 spec 和至少一个 PRD 需求/验收编号。
- 先运行最小相关检查，再运行阶段要求的完整检查。
- 不得声称未实际执行的测试通过。
- 缺失工具链或存在前置失败时，必须写入任务 result 的 `unresolved_issues` 或 QA 报告的 Findings。
- 核心规则优先通过可观察行为测试，不以内部实现细节替代验收。

### 7.2 当前可用前端检查

在改动 `src/` 时至少运行：

```bash
npm run build
```

项目尚未配置 ESLint 或前端单元测试。桌面端发布构建命令为：

```bash
npm run tauri:build
```

相关工具确定后，Planner 必须先更新本文件、`.specflow/config.yaml` 的 `quality.test_commands` 和目标 Delta Spec，再把命令作为完成定义。

### 7.3 后续平台实现的最低测试范围

- Key 规范化、唯一性和检索排序单元测试。
- 事务回滚、Schema 迁移、数据库密钥错误和 revision 冲突测试。
- KGlobalAccel 冲突、剪贴板失败和 X11 自动粘贴降级测试。
- D-Bus AppMenu 注册、重连和无小部件降级测试。
- 手动/自动清理回收站及失败回滚测试。
- 10,000 条片段性能基准。

## 8. 文档与状态源

```text
.agent/
  AGENT.md                 项目宪法（本文件，稳定规则）
  master_plan.md           Phase 1～9 历史总览（只读归档）
  specs/01～19-*.md        Phase 1～9 历史 spec（只读归档）
.specflow/
  config.yaml              流程策略源（测试命令、修复策略、路由、默认可写路径）
  state.yaml               派生项目索引（runtime 生成，勿手改）
  system-specs/            已实现、已确认的系统行为（归档时由 Delta 合并生成）
  changes/<change-id>/     进行中的变更：proposal、specs/（Delta）、contracts/、design、
                           ux-design、tasks、state.yaml、handoffs/、reviews/、test_report
  history/                 已归档 change
docs/design/               设计宪章、tokens、视觉验收清单
```

- `docs/prds/prd.md`：产品意图与验收编号源；`.specflow/system-specs/` 形成后，它是“当前已实现行为”的事实源，PRD 只保留意图与版本历史。
- 单个 change 的 `state.yaml` 是该 change 的权威状态；聊天结论、`master_plan.md` 不是。
- `contracts/` 保存每个 spec 的详细契约（允许/禁止修改范围、IPC、数据契约、失败场景、测试点），与 Delta Spec 一起作为实现与 QA 输入。
- SpecFlow CLI：`node /home/ray/Projects/CodeRepo/codex-spec-first-workflow/dist/cli.js <command>`（下文记作 `specflow`）。

## 9. 角色边界

### Human / Orchestrator

- 由人类承担；同时担任 `operator_runtime`（运行 `specflow` 命令推进状态）。
- 只在四处介入：澄清需求、批准计划（`specflow approve`）、处理 `decision_required`（`specflow resolve`）、批准归档（`specflow archive`）。
- 执行无法自动化的实机验收（见 §10），或明确授权 Agent 执行。

### Planner（原 Master）

- 阅读 PRD、本文件、`docs/design/` 与 System Spec。
- 只允许修改 `.specflow/changes/<id>/` 的规划工件（proposal、specs、contracts、design、ux-design、tasks）与 `docs/design/`、`docs/prds/`。
- 禁止修改业务代码；不得自行推进状态。

### Implementer（原 Coder）

- 只执行任务包 `handoffs/<task-id>.yaml` 列出的 spec，读取其 inputs 与对应 `contracts/` 文件。
- 只修改任务包 `allowed_paths` 内的文件；contracts 中“仅限”约束同样有效。
- 完成后写 `handoffs/<task-id>.result.yaml`（`changed_files`、`test_results` 必填），并勾选 `tasks.md` 对应条目；不得声称 QA 通过。
- 修复轮只处理最新 QA 报告列出的 Findings。

### Reviewer（原 QA）

- 阅读 Delta Spec、contracts、真实 diff 与测试证据；默认只审查，不改业务代码。
- 填写 `reviews/<task-id>.md`（Design Impact ≠ none 时必须完成 Design Conformance 节），在 result 中给出 `verdict: pass | repair | escalation` 与 `category`。

### Subagent

- 默认禁止。只有 orchestrator 用 `specflow dispatch-subagent` 派发的只读辅助任务可以执行，不得写文件或再派生。

## 10. 变更与验收规则

每个 change 必须：

- 在 `proposal.md` 声明 Design Impact（`none` / `visual` / `design_system`）；后两档实施前必须有 `ux-design.md` 与视觉证据。
- 每个 Delta Spec 用 `ADDED / MODIFIED / REMOVED Requirements` 表达行为变化，Requirement 用 MUST 陈述，并附 GIVEN/WHEN/THEN 场景，关联 PRD 需求与验收编号。
- 每个 spec 是用户可观察的 vertical slice，有明确允许修改路径（`specflow add-spec --path`）与禁止范围（contracts）。
- 写出平台调用、事务边界、错误码和降级行为（contracts）。

状态流转由 runtime 管理（详见 SpecFlow `01-methodology/state-machine.md`）：

```text
planning → awaiting_plan_approval → ready → implementing ⇄ reviewing ⇄ repairing
        → awaiting_archive_approval → archived     （任一非终态可进入 blocked / decision_required）
```

- 自动修复：`implementation_defect`、`missing_test`、`regression`、`visual_defect`、`visual_regression`、`accessibility` 每个 spec 最多 2 次；需求歧义、架构、安全、数据迁移、权限不足立即升级给人。
- 实机验收约定：KDE 全局快捷键、写真实剪贴板、自动粘贴、外部应用粘贴、实机截图对照与 KDE Global Menu 显示不得由 Agent 自动执行（除非人明确授权）。Reviewer 在 QA 报告中把这些项标为 `pending_human`，不阻止 `pass`；人在归档审批前执行并记录于 change 的 `test_report.md`，失败时 `specflow reopen-qa` 退回对应 spec。
- 同一时间只允许一个活跃 change（`max_active_changes: 1`），spec 串行执行。

## 11. 完成定义

一个 spec 只有同时满足以下条件才算完成（`qa_passed`）：

- Delta Spec 场景与 contracts 中的功能和失败路径已实现。
- 相关测试已通过并写入 result，或无法执行的原因已写入 `unresolved_issues`。
- Reviewer 结论为 `pass`，QA 报告已导入。
- 没有未记录的范围扩张、数据风险或平台兼容风险。

一个 change 只有在全部 spec `qa_passed`、`test_report.md` 填写完毕（含实机项）、人工批准归档并完成 Delta 合并后才算完成。
