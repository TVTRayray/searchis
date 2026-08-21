# SPEC-15 回收站操作与检索索引一致性

## 基本信息

- 当前状态：`done`
- 当前责任角色：`Orchestrator`（已收口）
- Coder 实现：工作树内 `SearchIndex::remove` + 回收站五条路径定向同步 + 单元/db 回归测试（62 tests passed, clippy/build 干净，QA passed）
- 类型：发布阻塞后处理项
- 关联 PRD：`FR-SCH-02`（稳定排序、软删除不进入正常索引）、`FR-MGT-04`（回收站）、`AC-18/19`
- 前置 Spec：无（独立）
- 允许修改：`src-tauri/src/search_index.rs`、`src-tauri/src/storage.rs` 的回收站操作与索引同步、及其 `#[cfg(test)]` 测试
- 禁止修改：前端 `src/**`、`src/api/**` IPC 契约、数据库 Schema（`snippets` 表结构）、`docs/prds/**`、平台适配层（`platform/**`、KGlobalAccel、X11、AppMenu、剪辑板）

## 背景 / 根因（已由 QA 在 SPEC-13 实机确认）

`SearchIndex` 是内存检索索引，数据派生自 `snippets` 表，可全量重建。当前只有 create/update/record_usage 路径调用索引 `upsert`；而：

- `trash_move` 在数据库中设置 `deleted_at`，但内存索引项仍为 `deleted_at: None` → 检索仍命中该软删除记录（`search()` 按 `item.deleted_at.is_none()` 过滤，读的是旧内存值）。
- `trash_restore` 清除 `deleted_at`，但索引未同步（副本保持陈旧值，影响较少，仍需一致）。
- `trash_purge_one` / `trash_empty` / `auto_purge` 物理删除数据库行，但从未从 `SearchIndex` 移除 → 永久删除后快速搜索仍返回旧记录（QA 实机复现：隔离临时数据库上永久删除后搜索仍返回该片段）。

`SearchIndex` 无 `remove` 方法，`upsert` 只能按 id 更新或新增，无法移除。

## 目标

让回收站全部写操作与内存检索索引保持单一真相一致：软删除后被正常检索排除，还原后恢复，永久删除后不再返回。

## 成功标准

- [ ] 软删除（`trash_move`）后：同一进程内检索不再返回该片段；还原后重新返回。
- [ ] 永久删除单条（`trash_purge_one`）、清空回收站（`trash_empty`）、自动清理（`auto_purge`）后：检索不再返回对应片段。
- [ ] 数据库与内存索引在事务提交成功后同步；事务回滚不得留下半同步索引。
- [ ] 不改变 IPC 命令签名、领域类型、数据库 Schema 与前端行为。
- [ ] 新增索引一致性回归测试，断言上述五种回收站路径后 `search` 的可见性变化。

## 非目标

- 不实现跨进程/多连接索引（单进程单用户，Mutex 单索引已够）。
- 不重构 `SearchIndex` 的整体排序或索引结构。
- 不处理 KDE AppMenu（独立 SPEC-16）。

## 接口契约

- 新增 `SearchIndex::remove(&mut self, id: &str) -> bool`：按 id 从 `items` 移除，返回是否命中。
- `trash_move`：事务提交后 `upsert(&updated)`（updated 含正确 `deleted_at`）。
- `trash_restore`：事务提交后 `upsert(&updated)`（updated 含 `deleted_at: None`）。
- `trash_purge_one`：事务提交后 `remove(id)`（或对该 id 触发定向同步）。
- `trash_empty`：事务提交后移除全部被删 id；实现可收集实际删除 id 再逐一 `remove`，或在事务成功后 `refresh_search_index()`。
- `auto_purge`：同 `trash_empty`，按实际删除集合同步。
- 事务失败/回滚时不改动索引。

> 实现取向（懒且正确）：优先定向 `upsert`/`remove`，避免无谓全量重建；若因集合收集繁琐，允许在 `trash_empty`/`auto_purge` 成功后调用既有 `refresh_search_index()` 全量重建（10,000 条内开销可接受），但 `trash_move`/`trash_restore`/`trash_purge_one` 必须定向同步，不得全量重建。

## 数据变化

- 无 Schema/字段变化；仅内存索引生命周期内一致性修复。

## 失败场景

- 事务提交失败 → 返回错误码，索引不变（不产生半同步态）。
- 目标 id 不在索引中 → `remove` 返回 false，视为幂等，不报错。
- 软删除后再永久删除 → move 后 upsert（deleted_at=Some），purge 时 remove 命中并移除。

## 已冻结验收重开说明

本 slice 触碰回收站路径，按「Frozen Passed 仅 Master 记录该路径被修改时才重开」规则，**重开回收站行为验收（SPEC-05 的删除/还原/永久删除/清空路径）**，但只重开“检索可见性”相关断言，不重审 CRUD 其余已冻结契约。Coder 不得扩大验证范围。

## 测试点

- 单元（`search_index.rs` / `storage.rs` `#[cfg(test)]`）：
  - `trash_move` 后 `search` 不含该 id；
  - `trash_restore` 后 `search` 重新包含；
  - `trash_purge_one` / `trash_empty` / `auto_purge` 后 `search` 不含被删 id；
  - `remove` 对不存在 id 幂等返回 false；
  - 事务回滚路径不污染索引（如已有回滚测试则补断言）。
- 既有 55 tests 不得回归。

## Coder 实现记录

- 新增 `SearchIndex::remove(&mut self, id: &str) -> bool`：按 id 从 `items` 移除，命中返回 true；对不存在 id 幂等返回 false。
- `trash_move`：事务提交成功后 `upsert(&updated)`（updated 含正确 `deleted_at`）。
- `trash_restore`：事务提交成功后 `upsert(&updated)`（updated 含 `deleted_at: None`）。
- `trash_purge_one`：事务提交成功后 `search_index.remove(id)`。
- `trash_empty` / `auto_purge`：事务提交前用新增 `collect_snippet_ids` 在事务内收集待删 id，提交成功后逐一 `index.remove(id)`，定向同步，不做全量重建。
- 事务失败/回滚路径不触碰索引（索引同步仅在 `transaction.commit()` 成功之后执行）；单个 `remove`/`upsert` 对整个 Mutex 索引持锁，避免半同步态。
- 关键发现：id 集合必须在 DELETE 行**之前**收集，否则 `collect_snippet_ids` 返回空（初版测试即暴露此问题）。

### 测试（全部新增于 `#[cfg(test)]`）

- `search_index.rs`：`remove_removes_by_id_and_is_idempotent`、`remove_missing_id_returns_false_without_change`。
- `storage.rs`：`trash_move_excludes_id_from_search_index`、`trash_restore_reincludes_id_in_search_index`、`trash_purge_one_removes_id_from_search_index`、`trash_empty_removes_ids_from_search_index`、`auto_purge_removes_ids_from_search_index`（后者用原始 UPDATE 置 deleted_at 复现“索引持有陈旧值”场景，覆盖修复前永久删除后仍返回旧记录的根因）。

### 实际执行的检查

- `cargo test --manifest-path src-tauri/Cargo.toml`：62 passed, 0 failed（原 55 + 新增 7，无回归）。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`：通过，无告警。
- `cargo build --manifest-path src-tauri/Cargo.toml`：通过。
- 改动范围仅限 `src-tauri/src/search_index.rs` 与 `src-tauri/src/storage.rs`（`git diff --stat` 确认 158 insertions，无前端/IPC/commands.rs/平台代码改动）。

### 残余风险

- `trash_empty`/`auto_purge` 定向收集删除使用新的 `collect_snippet_ids` 辅助函数；未额外覆盖“空回收站”边界（`trash_empty` 对空集合同样幂等，无索引变更）。
- `refresh_search_index()` 仍为 `#[cfg(test)]`，生产代码不依赖全量重建；本次采用定向同步，无线上全量重建开销。
- 单一 Mutex 索引，单进程单用户，符合非目标约束。

## QA Result

- Status：`passed`
- Owner Back：`none`
- Verdict Date：2026-08-21
- Summary：回收站五条写路径与检索索引一致性成立，7 项新增测试全部通过，既有 55 tests 无回归（总计 62 passed），clippy/build 干净。
- Scope Review：仅修改 `src-tauri/src/search_index.rs`（+27）和 `src-tauri/src/storage.rs`（+131），共 158 insertions，未触碰前端/IPC 契约/Schema/platform/commands.rs 签名。
- Contract Verification：`trash_move`/`trash_restore` 提交后 `upsert(updated)` ✅；`trash_purge_one` 提交后 `remove(id)` ✅；`trash_empty`/`auto_purge` 提交前收集 id、提交后逐一 `remove` ✅；`remove` 对不存在 id 幂等 ✅；事务回滚不污染索引（同步仅在 commit 成功后执行）✅。
- Findings：无。
- Risks：`collect_snippet_ids` 使用 format! 拼接 WHERE 条件，但 condition 均为调用方硬编码常量，无 SQL 注入风险；单一 Mutex 索引，单进程单用户，符合非目标约束。
- Missing Tests：无必要缺失。`trash_empty` 空集合边界由幂等 `remove` 覆盖；`auto_purge` 用原始 UPDATE 复现“索引持有陈旧值”场景，覆盖修复前根因。
- Required Fixes：无。
- Checks：`cargo test` 62 passed, 0 failed ✅；`cargo clippy --all-targets -- -D warnings` 无告警 ✅；`cargo build` 通过 ✅。
- E2E：本次为纯 Rust 层修复，不涉及前端 UI/E2E 路径；现有 E2E 在上一轮已验证回收站 UI 行为。

## 完成定义

回收站五条写路径与检索索引一致性成立、相关单元测试通过、既有测试无回归、`cargo test`/`clippy`/`cargo build` 通过；QA `passed`，状态回写完成。
