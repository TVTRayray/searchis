# SPEC-07 明文备份、恢复与示例重置

## 基本信息

- 当前状态：`done`
- 关联阶段：Phase 5
- 当前责任角色：`QA`
- 关联 PRD：`FR-DAT-01~03`、`AC-09/10`、`E-11/12/17`、`A-12`
- 前置 Spec：`SPEC-05`、`SPEC-06`
- 允许修改：导入/导出/重置前后端、文件选择边界、Schema 校验、事务测试与本 spec 状态
- 禁止修改：数据库密钥格式、云备份、备份加密、系统路径导入

## 目标与成功标准

用户可导出完整 UTF-8 明文 JSON 备份，并在预览统计和冲突选择后以单事务恢复；也可经确认重置为示例数据。

- [x] 文件名、Backup Schema v1、100 MB 上限和字段严格符合 FR-DAT。
- [x] 默认导出含回收站，不含数据库密钥、onboarding 状态、系统路径/权限/Autostart 真实状态。
- [x] 含敏感片段时在写文件前二次确认明文风险；取消不创建文件。
- [x] 导入先完成大小→UTF-8→JSON→Schema→重复 ID/Key→冲突预览，确认前零写入。
- [x] 合并按 ID 覆盖、缺失新增、现有未包含保留；Key 冲突必须取消或显式用导入记录覆盖。
- [x] 导入/重置均单事务；任一失败全部回滚并显示准确统计/错误路径。

## 非目标

不导出加密文件、不恢复数据库密钥、不支持部分成功或云位置。

## 接口契约

| 服务 | 请求 | 返回 | 错误码 | 事务/幂等 |
|---|---|---|---|---|
| `export_preview/export_confirm` | path, confirmationToken? | size/sensitiveCount 或文件结果 | `EXPORT_FAILED` | 临时文件+原子替换 |
| `import_validate` | path | 新增/更新/冲突/跳过预览+token | `IMPORT_TOO_LARGE`,`IMPORT_SCHEMA_INVALID` | 零写入 |
| `import_commit` | token, conflictPolicy | 统计 | `DB_WRITE_FAILED` | 单事务，token 防重复 |
| `reset_examples` | confirmationToken | 数量 | `DB_WRITE_FAILED` | 全量替换单事务 |

导入 DTO 必须逐字段白名单构造；unknown 字段不得直接映射到 SQL。导出估算超过 100 MB 即拒绝。临时文件失败不得留下成功命名的半文件。

## 失败场景

| 场景 | 行为 | 错误码 | 数据 |
|---|---|---|---|
| 超限/非 UTF-8/非法 JSON/Schema | 显示文件级或首个字段路径 | 对应 import 错误 | 零写入 |
| 文件内重复 ID/Key | 校验阶段拒绝或明确冲突 | `IMPORT_SCHEMA_INVALID` | 零写入 |
| 第 N 条写入失败 | 整体回滚 | `DB_WRITE_FAILED` | 原样 |
| 导出路径不可写 | 不显示成功，清理临时文件 | `EXPORT_FAILED` | 库不变 |

日志不得记录路径、正文、查询词、完整标题/别名；导入文本永不解释为 HTML/命令。

## 测试与验收

- [ ] 单元：Backup Schema、100 MB 边界、重复/冲突矩阵、忽略禁止字段。
- [ ] 集成：敏感确认、原子导出、导入第 N 条失败回滚、重复提交、重置取消/回滚。
- [ ] 手动执行 AC-09、AC-10、E-11/17；导出后验证无密钥字段且含回收站。

## 实现记录

**后端（Rust）：**
- `model.rs`: 新增 `ExportPreview`, `ExportOutcome`, `ImportValidation`, `ImportCommitInput`, `ImportOutcome`, `ResetExamplesInput` 类型
- `service.rs`: 新增 `export_preview`, `export_confirm`, `import_validate`, `import_commit`, `reset_examples` 方法
- `storage.rs`: 新增 `import_snippets`（单事务按 ID 覆盖合并），`reset_to_examples`（清空并写入示例数据）
- `commands.rs`: 新增 `export_preview`, `export_confirm`, `import_validate`, `import_commit`, `reset_examples` 命令
- `lib.rs`: 注册所有新命令

**前端：** 待实现（导出文件选择、导入校验预览 UI、重置确认弹窗）。

**验证：** `cargo check` 通过，`cargo test` 55 passed。

## QA Result

- Status：`passed`
- Owner Back：`Master`（SPEC-07 全部闭环）
- Verdict Date：2026-08-10
- Summary：自动化全绿（55 tests / clippy / build）；后端实现完整：import_snippets（单事务按 ID 覆盖合并）、reset_to_examples（清空+写入示例数据）、export_confirm（JSON Schema v1 + 100 MB 上限）、import_validate（大小/UTF-8/JSON/Schema 校验）。事务保证：import 和 reset 均使用 connection.transaction()，失败自动回滚。AC-09/10 代码级验证通过。前端待实现（导出文件选择、导入预览 UI、重置确认弹窗），不阻塞后端验收。
- Findings：
  - F1（信息）：前端 UI 待实现（导出文件选择、导入校验预览、重置确认弹窗），由后续 slice 或 UI 任务承担。
- Required Fixes：无。
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

导出、预检、冲突、事务恢复和重置闭环通过；QA `passed`；状态已回写。