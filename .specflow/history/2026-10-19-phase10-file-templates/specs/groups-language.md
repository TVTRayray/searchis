# Delta Spec: groups-language

> Change: `phase10-file-templates`
> Status: `draft`
> 原 SPEC-21。详细契约（IPC 表、数据契约、迁移、失败场景、测试点）：[`contracts/21-groups-and-language.md`](../contracts/21-groups-and-language.md)，实现与 QA 必读。
> PRD：A-15、FR-SNP-01（`language`、`groupId`）、FR-GRP-01、FR-GRP-02、FR-MGT-01（类型筛选）、FR-VIS-02、8.1、8.1.1、AC-21/22（不含目录部分）、AC-23、D-13。

## ADDED Requirements

### Requirement: Schema v4 迁移

数据库必须（MUST）在单事务内从 v3 迁移到 v4：新增 `groups` 表，`snippets` 新增 `group_id`（可空）、`language`（默认 `plain`）、`kind`（默认 `text`）；迁移前生成同目录、权限 `0600` 的 `searchis.db.v3.bak`；失败时回滚、原库与备份均保留，诊断日志不含正文。

#### Scenario: 已有片段迁移

- **GIVEN** v3 库中有 100 条片段
- **WHEN** 新版本首次启动
- **THEN** 100 条片段均为未分组、`language=plain`、`kind=text`，且存在 `searchis.db.v3.bak`

#### Scenario: 迁移失败回滚

- **GIVEN** 迁移过程中注入写入失败
- **WHEN** 应用启动
- **THEN** 进入只读错误页，原库内容不变，`.v3.bak` 存在，错误码 `DB_WRITE_FAILED`

### Requirement: 分组树管理

用户必须（MUST）能在管理窗口左栏新建、重命名、移动、删除分组：名称非空、不含 `/`、不为 `..`、不以 `.` 开头、≤ 64 字符，否则 `GROUP_NAME_INVALID`；同级名称不区分大小写唯一，否则 `GROUP_NAME_CONFLICT`；树深最多 3 层，否则 `GROUP_DEPTH_EXCEEDED`；不得移动到自身子孙下。每个写操作单事务，移动时整棵子树重算深度。

#### Scenario: 超过 3 层被拒绝（AC-21 数据部分）

- **GIVEN** 已有分组 `运维 > 数据库`
- **WHEN** 用户在 `数据库` 下新建 `慢查询`，再尝试在 `慢查询` 下新建子分组
- **THEN** `慢查询` 创建成功；第二次新建被拒绝并显示 `GROUP_DEPTH_EXCEEDED`

#### Scenario: 删除非空分组（AC-22 数据部分）

- **GIVEN** 分组 `行政` 含 2 个条目和子分组 `报销`
- **WHEN** 用户删除 `行政` 并确认
- **THEN** 2 个条目与 `报销` 在同一事务内上移到父级（未分组），与父级重名的子分组加 `(2)`；取消确认时一切不变

### Requirement: 左栏结构

管理窗口左栏必须（MUST）自上而下为：全部、置顶、最近使用；可折叠分组树（带“+”、含“未分组”、显示条目数）；可整体折叠的标签区；回收站；底部按钮栏。

#### Scenario: 分组计数

- **GIVEN** 分组 `运维` 含 3 条片段
- **WHEN** 打开管理窗口
- **THEN** 分组树中 `运维` 显示条目数 3，“未分组”显示其余条目数

### Requirement: 文本片段语言与高亮

文本片段必须（MUST）支持 `language ∈ {plain, bash, sql, yaml, json, toml, ini}`，未知值以 `VALIDATION_FAILED` 拒绝；编辑区为 CodeMirror 6，按语言高亮并使用语法高亮 token；中文输入法、Ctrl+Z、粘贴、Tab 行为正常；语言只影响显示，复制/粘贴结果与正文逐字节一致；敏感片段未展示时编辑器不挂载正文。

#### Scenario: 语言标记与高亮（AC-23）

- **GIVEN** 文本片段 `ngxr` 的语言为 `bash`
- **WHEN** 用户在管理窗口打开它，并在快搜窗口搜索 `ngxr`
- **THEN** 编辑区按 bash 高亮；两处列表均显示 `BASH` 徽标；复制结果与正文逐字节一致

### Requirement: 类型筛选与列表行

中栏列表顶部必须（MUST）提供“全部 / 文本 / 文件”类型筛选；每行显示类型图标、标题、Key，`language≠plain` 时显示语言徽标。本 spec 中“文件”筛选始终为空并显示通用空状态。

#### Scenario: 文件筛选为空

- **GIVEN** 尚未实现文件模板
- **WHEN** 用户选择“文件”筛选
- **THEN** 列表显示通用空状态

### Requirement: 快搜结果行 Q-A

快搜窗口结果行必须（MUST）为：左侧类型图标（按 language），中间标题，副标题 `Key · 分组 · 使用次数`，右侧徽标与 `Ctrl+N` 序号；匹配片段用 `--color-highlight` 高亮。检索排序（FR-SCH-02）不变，分组名不参与检索。

#### Scenario: 分组名不参与检索

- **GIVEN** 分组 `运维` 下有片段 `ngxr`，没有任何条目的 key/标题/标签包含“运维”
- **WHEN** 在快搜窗口输入 `运维`
- **THEN** 不返回 `ngxr`；搜索 `ngxr` 时副标题显示 `ngxr · 运维 · N 次`

### Requirement: 分组名隐私

日志必须（MUST）只记录分组 `id`，不记录分组名称。

#### Scenario: 重命名分组日志

- **GIVEN** 开启诊断日志
- **WHEN** 用户把分组 `财务` 重命名为 `报销`
- **THEN** 日志中不出现“财务”或“报销”

## Non-goals and Risks

- 非目标：磁盘目录同步与文件模板（file-templates）；备份格式（settings-backup）；条目拖拽排序；语言自动识别。
- 风险：settings-backup 完成前导出的 v1 JSON 备份不含分组与语言（已接受，分支内不发布）。
