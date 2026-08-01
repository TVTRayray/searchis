# SPEC-05 回收站完整生命周期

## 基本信息

- 当前状态：`todo`
- 关联阶段：Phase 4
- 当前责任角色：`Coder`
- 关联 PRD：`FR-MGT-04/05`、`AC-06/07/18/19`、`E-10/12`
- 前置 Spec：`SPEC-04`
- 允许修改：回收站相关前后端、清理调度、存储事务与测试、本 spec 状态
- 禁止修改：导入导出、KDE 系统集成、发布构建

## 目标与成功标准

用户可把片段移入回收站、还原或经确认永久删除；可手动清空，并按关闭/7/30/90 天自动清理。

- [ ] 软删除原子写 `deletedAt/revision`，立即从普通列表和检索隔离。
- [ ] 还原保持 id/内容，并受全表 normalizedKey 唯一约束；冲突时两条记录均不变。
- [ ] 单条永久删除和清空回收站必须二次确认，取消不写库。
- [ ] 清空在单事务完成并显示数量；任一失败全部回滚。
- [ ] 自动清理默认关闭，启动后一次、连续运行每 24h；阈值按 `>=`，0 条不通知。

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

- [ ] 单元：软删除隔离、阈值 29/30/31 天、关闭/7/30/90、系统时间边界。
- [ ] 集成：还原冲突、单条永久删除取消/确认、清空失败回滚、定时器不重复。
- [ ] UI：确认文案、取消零写入、删除数量、0 条无通知。
- [ ] 手动执行 AC-06、07、18、19。

## QA Result

- Status：`not_run`
- Owner Back：`none`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

生命周期和两类清理均具备事务/回滚证据；QA `passed`；状态已回写。