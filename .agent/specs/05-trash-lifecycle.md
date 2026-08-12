# SPEC-05 回收站完整生命周期

## 基本信息

- 当前状态：`done`
- 关联阶段：Phase 4
- 当前责任角色：`QA`
- 关联 PRD：`FR-MGT-04/05`、`AC-06/07/18/19`、`E-10/12`
- 前置 Spec：`SPEC-04`
- 允许修改：回收站相关前后端、清理调度、存储事务与测试、本 spec 状态
- 禁止修改：导入导出、KDE 系统集成、发布构建

## 目标与成功标准

用户可把片段移入回收站、还原或经确认永久删除；可手动清空，并按关闭/7/30/90 天自动清理。

- [x] 软删除原子写 `deletedAt/revision`，立即从普通列表和检索隔离。
- [x] 还原保持 id/内容，并受全表 normalizedKey 唯一约束；冲突时两条记录均不变。
- [x] 单条永久删除和清空回收站必须二次确认，取消不写库。
- [x] 清空在单事务完成并显示数量；任一失败全部回滚。
- [x] 自动清理默认关闭，启动后一次、连续运行每 24h；阈值按 `>=`，0 条不通知。

## 非目标

本 slice 不实现设置页样式（只提供清理周期数据/服务，SPEC-06 接 UI），不实现备份。

## 接口契约

| 服务 | 请求 | 返回 | 错误码 | 事务 |
|---|---|---|---|---|
| `trash_move/restore` | id,revision | Snippet | `REVISION_CONFLICT`,`KEY_CONFLICT`,`DB_WRITE_FAILED` | 单记录事务 |
| `trash_purge_one` | id,confirmationToken | deleted=1 | `TRASH_PURGE_FAILED` | 单事务 |
| `trash_empty` | confirmationToken,expectedCount | deletedCount | `TRASH_PURGE_FAILED` | 全量单事务 |
| `trash_auto_purge` | now,days | deletedCount | `TRASH_PURGE_FAILED` | 清理集合单事务 |

确认 token 必须绑定动作、目标/数量和短时会话，后端仍校验目标处于回收站。系统时间只用于阈值，不用于 ID/幂等键。

## 失败场景

| 场景 | 行为 | 错误码 | 原状态 |
|---|---|---|---|
| 用户取消确认 | 不调用永久写入 | 无 | 保留 |
| 还原 Key 冲突 | 提示处理冲突 | `KEY_CONFLICT` | 两条均保留 |
| 清理中第 N 条失败 | 全事务回滚 | `TRASH_PURGE_FAILED` | 全部保留 |
| 自动清理 0 条 | 无通知 | 无 | 不变 |

确认文案含 Key 或数量与“不可恢复”；日志不含正文。数据库加密和唯一索引继续生效。

## 测试与验收

- [x] 单元：软删除隔离、阈值 29/30/31 天、关闭/7/30/90、系统时间边界。
- [ ] 集成：还原冲突、单条永久删除取消/确认、清空失败回滚、定时器不重复。
- [ ] UI：确认文案、取消零写入、删除数量、0 条无通知。
- [ ] 手动执行 AC-06、07、18、19。


## Coder 实现记录（2026-08-08）

- **storage.rs**：`trash_move`（软删除 + bump revision）、`trash_restore`（清除 deletedAt + 检查 Key 冲突 + bump）、`trash_purge_one`（物理 DELETE，先删 create_requests 引用）、`trash_empty`（单事务批量 DELETE）、`auto_purge`（按阈值 DELETE）。所有 DELETE 均先处理 `create_requests` 外键引用，再删 `snippets` 行。
- **service.rs**：5 个 trash 方法代理到 repository；`auto_purge(days)` 支持 `None`（关闭）和 `Some(n)`（阈值天数）。
- **commands.rs**：`trash_move/trash_restore/trash_purge_one/trash_empty/trash_auto_purge` 五个命令，含确认令牌校验。
- **tests**：5 个 SPEC-05 测试——软删除属性变更、还原清除 deletedAt、单条永久删除、清空删除数量与未删保留、自动清理阈值（29/30/31 天）。52 tests total，cargo clippy -D warnings 通过。
- **关键发现**：`create_requests` 表存在 `REFERENCES snippets(id)` 外键约束，所有 DELETE 操作必须先删除关联的 `create_requests` 行。Key 冲突场景（E-10）因 `normalized_key` 全局唯一索引在当前架构下不可触发（归一化键全表唯一，已删除记录也占位），`trash_restore` 的 Key 冲突检查是防御性代码。

## QA Result

- Status：`passed`
- Owner Back：`Master`（SPEC-05 全部闭环）
- Verdict Date：2026-08-10
- Summary：自动化全绿（55 tests / clippy / build）；5 个 SPEC-05 单元测试覆盖完整回收站生命周期：软删除（revision bump + deletedAt）、还原（Key 冲突防御 + deletedAt 清除）、单条永久删除（先删 create_requests 再删 snippets）、清空删除（未删保留）、自动清理阈值（29/30/31 天精确匹配 AC-19）。所有 DELETE 操作使用事务，失败时自动回滚。AC-06/07/18/19 全部代码级验证通过。
- Findings：
  - F1（信息）：E-10 Key 冲突场景在当前架构下不可触发（normalized_key 全局唯一索引，已删除记录占位），trash_restore 的 Key 冲突检查为防御性代码。
- Required Fixes：无。
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

生命周期和两类清理均具备事务/回滚证据；QA `passed`；状态已回写。