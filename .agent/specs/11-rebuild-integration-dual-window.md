# SPEC-11 重建前端接入与双窗口形态落地

## 基本信息

- 当前状态：`todo`
- 关联阶段：Phase 4（重建整合）
- 当前责任角色：`Coder`
- 关联 PRD：`FR-PCK-01~03`、`FR-MGT-01~03`、`FR-SCH-01~03`、`FR-SNP-03`、`AC-01/05/08/13`、`E-06`
- 前置 Spec：`SPEC-01`（done）、`SPEC-02`（done）
- 允许修改：`src/**`、`src-tauri/**`（窗口创建与命令暴露）、`tauri.conf.json`、`package*.json`、本 spec 与 `master_plan.md`
- 禁止修改：`docs/prds/**`、`storage.rs` 的 Schema/加密/事务/校验逻辑（SPEC-01 已验收，只允许新增只读/粘贴命令的薄封装）

## 背景

重建前端（astryx 组件库、ManagerWindow/SettingsModal/OnboardingWizard/HeaderBar/KeyboardShortcutsModal、rofi 风格 QuickSearchWindow）已由 Master 文件级覆盖到仓库根目录 `src/`，`npm run build` 通过。但该重建处于 mock 形态：检索走 `utils/searchEngine.ts`、数据来自 `data/initialSnippets.ts`，ManagerWindow 等组件不调 IPC。当前 Rust 后端为 SPEC-01/02 已验收状态（SQLCipher 存储、search_index、clipboard.rs 均已实现且测试通过），前端与后端尚未接通。

同时，PRD 定义的应用形态是两个独立窗口：**检索窗口**（全局快捷键呼出的无边框置顶小窗口，rofi/krunner 式，Enter 复制/粘贴后关闭）+ **管理窗口**（常规主窗口，CRUD/筛选/设置）。当前 tauri.conf.json 只有一个常规窗口，重建前端把 QuickSearchWindow 做成单窗口内模态弹出层——视觉近似但非 OS 级独立窗口。本 spec 必须落地双窗口形态。

## 目标与成功标准

- [ ] 检索窗口成为独立 Tauri 窗口：无边框、置顶、窄小尺寸（rofi/krunner 式）、Esc 关闭、输入框自动聚焦。
- [ ] 管理窗口为独立常规窗口，包含 ManagerWindow/设置/向导视图；两个窗口互不嵌套。
- [ ] QuickSearchWindow 从真实后端检索：新增 `search_snippets` 命令（复用 search_index.rs），前端不再依赖 mock searchEngine 与 initialSnippets。
- [ ] 复制/自动粘贴回调接真实命令：新增 `copy_snippet` 命令（复用 clipboard.rs 与操作幂等），剪贴板失败显示错误且不计数。
- [ ] 管理窗口 CRUD 接 `snippet_create/update/get/list`（契约已一致），revision 冲突与字段错误可显示。
- [ ] 敏感片段遮挡、错误码展示与"原 front-baseline 无音效"约束不被重建破坏。
- [ ] `cargo test` 既有测试保持通过；`npm run build` 通过；`npm run tauri:build` 可产出可执行文件。

## 非目标

- 不实现 KGlobalAccel 全局快捷键与 X11 自动粘贴（SPEC-03）。
- 不实现设置持久化到 Rust（SPEC-06；重建中的 localStorage 主题逻辑在本 spec 保留即可，SPEC-06 迁移）。
- 不实现回收站、导入导出、向导完整流程（SPEC-05/07/08）。
- 不修改数据库 Schema、加密、事务与校验逻辑。

## 接口契约

| 命令 | 请求 | 成功返回 | 错误码 | 说明 |
|---|---|---|---|---|
| `snippet_list/get/create/update` | 既有契约 | 既有 | 既有 | 已存在，前端直接调用 |
| `search_snippets`（新增） | query, limit | items, total | `DB_OPEN_FAILED` | 薄封装 search_index.rs，仅未删除片段 |
| `copy_snippet`（新增） | id, operationId, keepOpen | copied 统计 | `CLIPBOARD_WRITE_FAILED`,`DB_WRITE_FAILED` | 薄封装 clipboard.rs，operationId 幂等 |

- 新增命令只允许调用已验收的 service/storage 能力，不得重写排序、事务或加密。
- 前端 UI 不得直接拼 SQL 或调用未封装平台命令。
- 检索窗口 `onClose` 对应真实窗口关闭；`Ctrl+E`/`Ctrl+N` 跳转管理窗口必须真实切换窗口并传递预填数据（用 Tauri 窗口间事件或先关后开 + 参数传递，二选一，需记录决策）。

## 失败场景

| 场景 | 预期 | 错误码 | 数据 |
|---|---|---|---|
| 数据库未初始化/密钥不可用 | 错误页显示，不进入 mock 数据 | `DB_KEY_UNAVAILABLE`,`DB_OPEN_FAILED` | 不写入 |
| 剪贴板写入失败 | 检索窗口保留并显示错误，不计数 | `CLIPBOARD_WRITE_FAILED` | 不更新 |
| IPC 契约字段不一致 | 前端显示错误，不崩溃 | `IPC_FAILED` | 不写入 |
| revision 冲突 | 提示重新载入（复用既有逻辑） | `REVISION_CONFLICT` | 不覆盖 |

## 测试与验收

- [ ] `npm run build`、`cargo test`、`cargo clippy`、`npm run tauri:build` 全绿。
- [ ] 两个独立窗口形态在 Arch/KDE/X11 下实机验证（检索窗口无边框置顶、Esc 关闭、聚焦）。
- [ ] 检索、复制、管理 CRUD 走真实数据，无 mock 残留（`grep -r initialSnippets src` 为空或仅测试数据）。
- [ ] 敏感正文不出现在默认可见文本与 a11y label。
- [ ] 手动回归 AC-01、AC-05、AC-08、AC-13。

## 实施记录

- Changed Files：
- Commands Run：
- Validation Output：
- Residual Risks：
- Implementation Notes：

## QA Result

- Status：`not_run`
- Owner Back：`none`
- Verdict Date：
- Summary：
- Findings：
- Risks：
- Missing Tests：
- Required Fixes：
- Retest Criteria：

## 完成定义

双窗口形态、真实数据闭环、命令薄封装与既有测试保持通过；QA `passed`；`master_plan.md` 状态一致。
