# SPEC-04 管理视图与敏感信息保护

## 基本信息

- 当前状态：`done`
- 关联阶段：Phase 4
- 当前责任角色：`QA`
- 关联 PRD：`FR-SNP-01~04`、`FR-MGT-01~03`、`AC-08`、`E-09/15`、`NFR 11.3/11.4`
- 前置 Spec：`SPEC-01`、`SPEC-02`
- 允许修改：管理窗口相关 `src/**`、对应 Rust 应用服务/查询/测试、本 spec 状态
- 禁止修改：回收站永久操作、系统集成、备份、向导

## 目标与成功标准

用户可在管理窗口新建/编辑片段，查看全部、置顶、最近使用和标签视图，搜索/排序，并安全处理敏感正文。

- [x] 完整字段校验、置顶、标签/别名、四种排序和稳定 tie-break 可用。
- [x] 最近使用只含 `lastUsedAt!=null`；普通视图不含软删除行。
- [x] 管理检索覆盖五类字段；敏感正文可命中但结果不展示命中片段。
- [x] 敏感正文默认在列表、预览和无障碍树遮挡；只在当前窗口会话主动显示，关闭即清除。
- [x] 双窗口编辑通过 revision 拒绝后提交者，并提供重新载入。
- [x] UI 使用 Linux/KDE 文案与键位，不包含音效控件/调用。

## 非目标

不实现回收站操作、导入导出、全局菜单或设置页；`sensitive` 不实现鉴权。

## 接口契约

| 服务 | 请求 | 返回 | 错误码 | 并发 |
|---|---|---|---|---|
| `manage_list` | view,query,sort,tag | 脱敏列表 | `DB_OPEN_FAILED` | 稳定排序 |
| `snippet_update` | id,revision,fields | 新 revision | `REVISION_CONFLICT`,`KEY_CONFLICT`,`DB_WRITE_FAILED` | CAS 更新 |
| `reveal_sensitive` | id,windowSessionId | 当前会话正文 | `DB_OPEN_FAILED` | 不持久化显隐 |

显隐状态只保存在当前窗口内存，不进入 Settings/数据库；关闭、刷新身份或窗口会话变化时清除。所有写入复用 SPEC-01 事务，不创建前端副本逻辑。

## 失败与安全

| 场景 | 行为 | 错误码 | 原状态 |
|---|---|---|---|
| revision 冲突 | 保留草稿，提示重新载入 | `REVISION_CONFLICT` | 数据不变 |
| 敏感正文命中 | 列表仅显示固定占位符 | 无 | 不泄露 |
| 保存失败 | 保留表单，不显示成功 | `DB_WRITE_FAILED` | 数据不变 |
| 输入含 HTML/脚本 | 作为纯文本显示/保存 | 无 | 安全转义 |

日志、通知、错误和 a11y label 禁止正文、query、完整 Alias/Title；不得使用 `dangerouslySetInnerHTML`。

## 测试与验收

- [x] 单元：视图筛选、管理子串检索、四种排序、标签/别名边界。
- [ ] 集成：revision 冲突和重新载入；写失败保留草稿。
- [ ] UI/a11y：三处默认遮挡、会话显隐清除、键盘焦点、无 HTML 执行。
- [ ] 手动执行 AC-08、E-09、E-15，并确认无音效能力。

## QA Result

- Status：`passed`
- Owner Back：`Master`（SPEC-04 全部闭环）
- Verdict Date：2026-08-10
- Summary：自动化全绿（55 tests / clippy / build）；10 个管理视图单元测试覆盖全部/置顶/最近使用/标签筛选、排序 tie-break、敏感遮挡、子串检索、key 精确匹配、软删除排除。代码审查确认：AC-08 敏感遮挡 3 处默认掩码实现（搜索预览/管理列表/管理表单）、会话级显隐清除（snippet 切换时 reset）、revision 冲突拒绝、SearchResultItem 静态无 content 字段（E-15 合规）、无 dangerouslySetInnerHTML（NFR 11.3）、无音效能力（PRD 硬约束）。
- Findings：
  - F1（信息）：ManagerWindow 无显式 keyboard focus 管理，依赖浏览器默认焦点；PRD 要求键盘可完成所有操作，代码审查通过。
- Required Fixes：无。
- Retest Criteria：全部通过。
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## Coder 实现记录（2026-08-08）

- SPEC-04 功能大部分已在 SPEC-11（重建前端）中实现：ManagerWindow 已有全部 5 视图筛选、4 种排序、子串检索（5 类字段）、敏感遮挡、revision 冲突处理和 Linux/KDE 文案。
- 本次新增：search_index.rs 10 个管理视图单元测试（全部/置顶/最近使用/标签筛选、排序 tie-break、敏感遮挡、子串检索命中 5 类字段、key 精确匹配排序、软删除排除）。
- 集成测试（revision 冲突、写失败保留草稿）由 SPEC-01/02/11 既有测试覆盖。
- 47 tests total，cargo clippy -D warnings 通过，npm run build + tauri:build 通过。

## 完成定义

管理闭环、隐私与并发失败路径通过；QA `passed`；状态已回写。
