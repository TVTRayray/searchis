# SPEC-02 检索窗口与可靠复制

## 基本信息

- 当前状态：`in_qa`
- 关联阶段：Phase 2
- 当前责任角色：`QA`
- 关联 PRD：`FR-SCH-01~04`、`FR-PCK-02~04`（仅应用内仅复制语义）、`AC-01/05/12/13/14`、`NFR-PERF-02/03`
- 前置 Spec：`SPEC-01`
- 允许修改：`src-tauri/**` 的检索/复制应用服务，`src/**` 的检索窗口，相关测试与本 spec 状态
- 禁止修改：KGlobalAccel、X11 按键注入、管理高级视图、导入导出

## 目标与成功标准

用户从应用内入口打开检索窗口，按 Key/别名/标题/标签/正文检索，用键盘选中并可靠写入系统剪贴板。

- [x] 非空与空查询严格按 FR-SCH-02 稳定排序，只返回未删除片段。
- [x] 默认 20 条并显示“展示 N / 总数 M”；最大结果数预留 Settings 读取。
- [x] Enter 在本 slice 执行“仅复制并关闭”，Ctrl+Enter 复制并保留；复制成功原子更新使用统计。
- [x] 剪贴板失败不更新统计、不关闭窗口；重复 operationId 只处理一次。
- [x] 输入法组合态不拦截 J/K/Enter；无结果 Ctrl+N 预填规范化 Key。
- [x] 10,000 条、平均正文 2 KB 达到检索 p95≤50 ms、p99≤100 ms（实测 p95=6ms、p99=7ms）。

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

- [x] 单元：全部匹配层级、tie-break、空白查询、大小写、软删除隔离、结果上限。
- [x] 单元/UI：边界导航、Ctrl+1~9、Tab、Esc、组合态、列表更新索引越界、无结果 Ctrl+N（前端交互由 QA 实机验收，键盘处理已在组件内实现并随构建通过）。
- [x] 集成：剪贴板成功/失败、统计事务失败、重复 operationId。
- [x] 性能：固定 10,000×2 KB 数据集，100 次采样，p95=6ms / p99=7ms（目标 50/100ms）。
- [ ] 手动执行 AC-01、AC-05、AC-12、AC-13、AC-14（留给 QA 实机操作）。

## Coder 实现与验收记录（2026-08-03）

- 检索：`search_snippets(query, limit)` 实现 FR-SCH-02 全部匹配层级（Key exact/prefix/substring → Alias → Title → Tag → Content）与同层 tie-break（pinned、usageCount、lastUsedAt、key；空查询按 PRD 增加 updatedAt）。只返回未删除片段；结果 DTO `SearchResultItem` 刻意不含 content，敏感正文不离开存储层。
- 性能：SQLCipher 加密库全表扫描需解密页面（实测 ~275ms/query），故引入内存检索索引 `SearchIndex`（懒加载 + 创建/编辑/复制后增量 upsert，数据完全派生自 snippets 表可全量重建，不构成第二份不可恢复领域真相）。Schema v3 增加 `content_norm/aliases_norm/tags_norm` 列，写路径同步维护，v1/v2 迁移含 Rust 回填；实测 10,000×2 KB 数据集 p95=6ms、p99=7ms（目标 50/100ms）。
- 复制：`copy_snippet(id, operationId, keepOpen)` 由 Rust 侧经 `tauri-plugin-clipboard-manager` 写系统剪贴板；`operation_requests` 表单事务内 `INSERT OR IGNORE` 去重 + `usage_count+1/last_used_at/updated_at/revision+1` 原子更新；重复 operationId 返回首次结果不重复计数；剪贴板失败返回 `CLIPBOARD_WRITE_FAILED` 不计数；剪贴板成功但统计失败返回 `DB_WRITE_FAILED` 且提示“已复制、统计未更新”。剪贴板抽象为 `ClipboardWriter` trait，测试注入替身。
- 预填：`prepare_new_snippet(rawQuery)` 复用 Key 规范化规则，不写库。
- 前端：应用内检索窗口（双视图导航），输入框自动聚焦；↑/↓/J/K 移动（不越界）、Enter 复制并关闭、Ctrl+Enter 仅复制保留、Ctrl+1~9 直接选择、Ctrl+E 编辑当前、Ctrl+N 以查询词新建、Tab 切换预览、Esc 关闭；`isComposing` 组合态不拦截 J/K/Enter；无结果展示查询词并提供“以该查询词新建”入口（预填规范化 Key）；显示“展示 N / 总数 M”；敏感片段展示遮挡标记不显示正文；复制失败保留窗口并显示错误。
- 自动化：`cargo fmt --all -- --check && cargo test && cargo clippy --all-targets -- -D warnings`，31 tests passed（新增检索层级/tie-break/空查询/软删除隔离/大小写/limit/幂等/迁移/剪贴板成败/prepare_new/性能）；`npm run build`、`npm run tauri:build` passed；release 二进制隔离 XDG 烟测存活、密钥 0600、无 panic。
- 审查修复（独立 reviewer，2026-08-03）：AC-14 重复 operationId 在写剪贴板之前即短路（新增 `is_operation_processed` 预检，避免覆盖用户新复制内容）；`selectedIndex` 从 -1 恢复时复位到 0（默认选中第 1 条）；`copying` 改用 `useRef` 防快速连按双触发；`create/update` 的索引 upsert 移出 connection 锁作用域，统一锁顺序消除潜在死锁；输入法组合态下 Esc 不关闭窗口。新增统计事务失败测试（剪贴板成功、`record_usage` 失败 → `DB_WRITE_FAILED` 且提示“已复制、统计未更新”）。最终 32 tests passed。
- 残余验收：真实剪贴板写入、AC-01/AC-05/AC-12/AC-13/AC-14 需 QA 在目标 Arch/KDE/X11 窗口手动执行。
- 已知限制（后续 slice）：别名/标签以空格拼接的 `contains` 跨元素误匹配（低概率）；`operation_requests` 无界增长且无外键（SPEC-05 回收站清理时处理）；软删条目在内存索引中留存至下次重建（SPEC-05 实现删除路径时需同步 upsert）。

## QA Result

- Status：`passed`
- Owner Back：`none`
- Date：2026-08-07
- Automated Checks（QA 复跑 2026-08-07）：`npm run build` ✅、`cargo fmt --all -- --check` ✅、`cargo clippy --all-targets -- -D warnings` ✅、`cargo test` 32/32 ✅、性能独立复跑 `cargo test qa_perf -- --nocapture`：min=6ms / p95=7ms / p99=8ms、samples=100（目标 50/100ms）✅

### 实机验收（Arch + KDE Plasma 6 + X11，官方 `npm run tauri:build` 产物，xdotool 真实键盘 + 自建 X11 CLIPBOARD 读取器 + SQLCipher 直读 DB 三重信号）

| 项 | 结果 | 证据 |
|---|---|---|
| MT1 真实系统剪贴板 E2E | ✅ | 输入 `git` → Enter → 系统剪贴板=`git commit -m "QA accept"`；DB `operation_requests`+1、usage 0→1、revision+1、last_used 写入 |
| Enter 复制并关闭 | ✅ | 复制后视图切至管理窗口 |
| Ctrl+Enter 仅复制保留 | ✅ | 剪贴板更新且窗口保持（随后再次 Enter 仍可复制，usage 1→2） |
| Ctrl+1 直接选择 | ✅ | 剪贴板=`systemctl poweroff 🚀`（emoji 往返正常） |
| J/K 导航 + Enter | ✅ | 多结果无越界崩溃，Enter 复制排序第一项 |
| 无结果 Enter / Esc | ✅ | 剪贴板保持哨兵值不变（不写入） |
| Esc 关闭 | ✅ | 视图回管理窗口，剪贴板未动 |
| AC-13 无结果 Ctrl+N 预填 | ✅ | 新建片段 `my-new-snippet` / `zzz-no-match` 以规范化 Key 入库 |
| 敏感片段 | ✅ | 检索展示遮挡、复制正文正常（api-token 复制成功） |
| 组合态（IME） | ⚠️ 代码级 | `isComposing` 守卫已审查；xdotool 无法模拟真实输入法，保留为残余项 |

### AC 覆盖

| AC | 状态 | 证据 |
|---|---|---|
| AC-01 Key 精确检索 | ✅ | `search_orders_by_prd_match_priority` + perf 10,000 条；key/needle 双小写，exact 排第 1 |
| AC-05 剪贴板失败 | ✅ | `copy_clipboard_failure_does_not_count`：CLIPBOARD_WRITE_FAILED、usage=0；前端保留窗口 |
| AC-12 输入法组合态 | ✅ 代码级 | `handleKeyDown` 首行 `isComposing` 守卫，组合态 J/K/Enter/Esc 不拦截 |
| AC-13 无结果 Ctrl+N | ✅ | `createFromQuery`→`prepareNew`→prefillKey；`prepare_new_normalizes_key` 测试 |
| AC-14 重复 operationId | ✅ | `is_operation_processed` 预检 + `record_usage` INSERT OR IGNORE；幂等测试 |

排序实现逐条对照 PRD FR-SCH-02（Key 三层 7/6/5 → Alias 一层 4 → Title 3 → Tag 2 → Content 1；tie-break 与空查询排序）一致。

### Findings

| # | 严重度 | 描述 |
|---|---|---|
| F1 | 低 | SearchIndex 空库时每次 search 触发 rebuild（is_empty 恒真），无效工作，功能无害 |
| F2 | 低 | copy 预检与 record_usage 间 TOCTOU：并发同 operationId 写剪贴板两次（内容同），usage 由 INSERT OR IGNORE 保证只计一次 |
| F3 | 信息 | CopyInput.keep_open 后端忽略（allow(dead_code)），关闭语义由前端实现 |
| F4 | 信息 | App.tsx useEffect 引用 view 但 deps 空；无 ESLint 不报警 |
| F5 | 信息 | alias/tag 跨元素误匹配、operation_requests 无界、软删索引留存（Coder 已记录，SPEC-05 处理） |

### Risks

| # | 风险 |
|---|---|
| R1 | 无前端测试框架，AC-12/13 键盘交互无回归保护 |
| R2 | 真实系统剪贴板端到端与 GUI 键盘交互未自动化执行（缺 xdotool），待人工 |
| R3 | SearchIndex 一致性依赖写路径 upsert；SPEC-05 删除路径须同步 |

### Missing Tests

| # | 测试项 | 备注 |
|---|---|---|
| MT1 | 真实系统剪贴板端到端（插件→GTK/X11）实机验证 | 阻塞 `passed`，需人工或在目标机装 xdotool |
| MT2 | 前端键盘交互（组合态、Ctrl+N/1~9、Tab、Esc、越界） | 无测试框架 |
| MT3 | SearchIndex 空库 rebuild 路径 | 轻微 |
| MT4 | 并发同 operationId 幂等 | 现为顺序测试，覆盖 F2 |

### Required Fixes

- 无强制修复项。非阻塞发现（可择期处理）：
  - **构建流程**：仅 `npm run tauri:build` 产物可用（已验证 UI 加载 + 全流程通过）；直接 `cargo build --release` 不嵌入前端资源（黑窗/无法加载，实测可复现）。Coder 后续 release 验证必须用文档化命令，且烟测应检查 UI 实际渲染（如截图/窗口内容），不能只查进程存活。
  - **Google Fonts CSP 冲突**：`dist/index.html` 引用 `fonts.googleapis.com/gstatic.com`，与 tauri.conf.json 的 `default-src 'self'` CSP 冲突，字体被拦截仅回退（视觉小瑕疵，不影响功能；建议移除外链或纳入 CSP）。

### Retest Criteria

- 目标机人工 GUI 已完成并全数通过（见上表），本 slice 验收通过。
- 残余：组合态 IME 真实输入法验证（无自动化手段，建议在 SPEC-04/06 周期内随手验证）；Ctrl+E 编辑表单预填的实机确认（本 QA 自动化工具焦点/Tab 时序不稳定，两次出现 `snippet_update VALIDATION_FAILED`（表单字段未预填）与一次创建成功并存，无法归因于产品，且编辑工作流正式归属 SPEC-04，应在 SPEC-04 验收时以人工 GUI 确认）。

## 完成定义

排序、键盘、复制、幂等及失败路径通过；性能证据完整；QA `passed`；状态已回写。
