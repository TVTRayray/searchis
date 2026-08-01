# SPEC-02 检索窗口与可靠复制

## 基本信息

- 当前状态：`todo`
- 关联阶段：Phase 2
- 当前责任角色：`Coder`
- 关联 PRD：`FR-SCH-01~04`、`FR-PCK-01~04`（不含全局注册/自动粘贴）、`AC-01/05/12/13/14`、`NFR-PERF-02/03`
- 前置 Spec：`SPEC-01`
- 允许修改：`front-baseline/src-tauri/**` 的检索/复制应用服务，`front-baseline/src/**` 的检索窗口，相关测试与本 spec 状态
- 禁止修改：KGlobalAccel、X11 按键注入、管理高级视图、导入导出

## 目标与成功标准

用户从应用内入口打开检索窗口，按 Key/别名/标题/标签/正文检索，用键盘选中并可靠写入系统剪贴板。

- [ ] 非空与空查询严格按 FR-SCH-02 稳定排序，只返回未删除片段。
- [ ] 默认 20 条并显示“展示 N / 总数 M”；最大结果数预留 Settings 读取。
- [ ] Enter 在本 slice 执行“仅复制并关闭”，Ctrl+Enter 复制并保留；复制成功原子更新使用统计。
- [ ] 剪贴板失败不更新统计、不关闭窗口；重复 operationId 只处理一次。
- [ ] 输入法组合态不拦截 J/K/Enter；无结果 Ctrl+N 预填规范化 Key。
- [ ] 10,000 条、平均正文 2 KB 达到检索 p95≤50 ms、p99≤100 ms。

## 非目标

不注册全局快捷键、不激活其他窗口、不发送 `Ctrl+V`；敏感正文高级展示由 SPEC-04 完成，本 slice 只确保结果 DTO 不泄露敏感摘要。

## 接口契约

| IPC / 服务 | 请求 | 返回 | 错误码 | 要求 |
|---|---|---|---|---|
| `search_snippets` | query, limit | items,total | `DB_OPEN_FAILED` | 参数化，只含未删除行 |
| `copy_snippet` | id,operationId,keepOpen | 统计和窗口动作 | `CLIPBOARD_WRITE_FAILED`,`DB_WRITE_FAILED` | operationId 幂等 |
| `prepare_new_snippet` | rawQuery | normalizedKey | `VALIDATION_FAILED` | 不写库 |

匹配层级严格为 Key exact/prefix/substring → Alias exact/prefix/substring → Title → Tag → Content；同层按 pinned、usageCount、lastUsedAt、key。空查询按 PRD 定义。正文作为纯文本写入，不读取剪贴板历史。

## 状态与事务

- 检索索引在创建/编辑/删除后原子更新或使用同库可重复查询；不得产生第二份不可恢复的领域真相。
- 剪贴板成功后，`operationId` 去重与 `usageCount+1/lastUsedAt/revision+1` 在单事务完成。
- 如果剪贴板成功但统计事务失败，报告 `DB_WRITE_FAILED` 与“已复制、统计未更新”，不得重复自动计数。

## 失败与隐私

| 场景 | 行为 | 错误码 | 保留窗口 |
|---|---|---|---|
| 剪贴板写入失败 | 不计数、不报成功 | `CLIPBOARD_WRITE_FAILED` | 是 |
| 无结果执行复制/编辑 | no-op | 无 | 是 |
| 敏感正文命中 | 返回片段但无正文摘要/无障碍正文 | 无 | 是 |
| operationId 重复 | 返回首次结果，usageCount 不重复 | 无 | 按首次语义 |

日志不得记录 query、content、剪贴板正文、完整 Alias/Title；错误提示包含对象、原因类别、下一步。

## 测试与验收

- [ ] 单元：全部匹配层级、tie-break、空白查询、大小写、软删除隔离、结果上限。
- [ ] 单元/UI：边界导航、Ctrl+1~9、Tab、Esc、组合态、列表更新索引越界、无结果 Ctrl+N。
- [ ] 集成：剪贴板成功/失败、统计事务失败、重复 operationId。
- [ ] 性能：固定 10,000×2 KB 数据集，目标环境至少 100 次，记录 p95/p99。
- [ ] 手动执行 AC-01、AC-05、AC-12、AC-13、AC-14。

## QA Result

- Status：`not_run`
- Owner Back：`none`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

排序、键盘、复制、幂等及失败路径通过；性能证据完整；QA `passed`；状态已回写。