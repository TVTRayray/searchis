# Delta Spec: file-templates

> Change: `phase10-file-templates`
> Status: `draft`
> 原 SPEC-22。详细契约（IPC 表、平台适配、数据契约 v5、扫描算法要求、失败场景、测试点）：[`contracts/22-file-template-management.md`](../contracts/22-file-template-management.md)，实现与 QA 必读。
> PRD：A-03、A-14、A-16、FR-SNP-01（`kind`、`file`）、FR-TPL-01～07、FR-GRP-02（目录同步）、FR-MGT-01、FR-MGT-04、FR-SCH-01、8.1 `TemplateFile`、9.3、AC-21/22（目录部分）、AC-24～29、E-21～24、E-27、NFR-PERF-07、D-11～D-15。

## ADDED Requirements

### Requirement: 模板目录与分组树一一对应

系统必须（MUST）维护 `<应用数据目录>/templates/` 与 `templates/.trash/`（权限 `0700`），首次运行为已有分组补建目录（包括只含文本片段的分组）；分组的新建、重命名、移动、删除同步修改对应目录；“未分组”对应 `templates/` 根目录。补建失败不阻止启动，文件模板功能显示 `TEMPLATE_FS_FAILED`。

#### Scenario: 新建子分组建目录（AC-21 目录部分）

- **GIVEN** 已有分组 `运维 > 数据库`
- **WHEN** 用户在 `数据库` 下新建 `慢查询`
- **THEN** 模板目录出现 `运维/数据库/慢查询/`

#### Scenario: 删除非空分组同步目录（AC-22 目录部分）

- **GIVEN** 分组 `行政` 含子分组 `报销`
- **WHEN** 用户删除 `行政` 并确认
- **THEN** 目录 `报销/` 移到模板目录根下，`行政/` 被删除，其中的文件随条目上移

### Requirement: Schema v5 文件模板字段

数据库必须（MUST）迁移到 v5，为 `snippets` 增加 `file_*` 字段并满足约束：`kind='file'` 时相对路径、扩展名、大小、哈希非空，`sensitive=0`，`content=''`；`kind='text'` 时 `file_*` 全为空；相对路径唯一。迁移前生成 `searchis.db.v4.bak`。

#### Scenario: v4 库升级

- **GIVEN** v4 库含分组与文本片段
- **WHEN** 新版本首次启动
- **THEN** 数据完整保留，存在 `searchis.db.v4.bak`，所有已有条目 `kind=text` 且 `file_*` 为空

### Requirement: 导入与新建文件模板

用户必须（MUST）能通过中栏“+”菜单导入文件、新建空白 md 或 xlsx，或把多个文件拖入当前分组导入。导入复制文件、不修改原文件，源路径不持久化；Key 由文件名生成（保留中文、非法字符换成 `-`、空结果为 `file`、冲突加 `-2` 等后缀）并标记“自动导入”，用户保存一次元数据后清除标记。单文件 > 20 MB 以 `TEMPLATE_TOO_LARGE` 拒绝；未删除条目合计不超过 10,000。多文件导入时被拒文件不影响其余文件。

#### Scenario: 导入文件并自动生成 Key（AC-24）

- **GIVEN** 已存在 Key `报销单`
- **WHEN** 用户把 `报销单.xlsx` 拖入分组 `行政`
- **THEN** 文件被复制到 `templates/行政/报销单.xlsx`，原文件不变；新条目 Key 为 `报销单-2`，带“自动导入”标记

#### Scenario: 超大文件（AC-25）

- **GIVEN** 一个 20 MB + 1 byte 的文件
- **WHEN** 用户导入它，或把它放进模板目录后打开管理窗口
- **THEN** 导入被拒绝并显示 `TEMPLATE_TOO_LARGE`；扫描跳过并提示；数据库与模板目录均无新增

### Requirement: 文件模板详情与外部打开

右栏选中文件模板时必须（MUST）显示元数据表单（标题、Key、分组、标签、置顶）、文件信息卡片（图标、文件名、大小、修改时间）、“打开”与“打开方式…”按钮；“导出”“带格式复制”可见但禁用；“放入回收站”在右上角；不显示敏感开关、不预览内容。标题与文件名双向联动，文件名清洗后重名加 ` (2)`。“打开”用 `xdg-open`，“打开方式…”列出支持该 MIME 的 `.desktop` 应用并按扩展名记住选择（`openWithByExtension`）；均以参数数组启动、不经 shell；失败显示 `TEMPLATE_OPEN_FAILED`；文件缺失时显示 `TEMPLATE_FILE_MISSING` 并触发一次扫描。

#### Scenario: 改标题同步文件名

- **GIVEN** 文件模板 `周报.md` 位于 `行政/`，同目录已有 `月报.md`
- **WHEN** 用户把标题改为 `月报` 并保存
- **THEN** 文件改名为 `行政/月报 (2).md`，标题显示 `月报 (2)`

#### Scenario: 打开方式记忆

- **GIVEN** 用户对某个 xlsx 选择“打开方式…”中的 WPS 表格
- **WHEN** 之后打开另一个 xlsx 模板的“打开方式…”
- **THEN** WPS 表格为默认选中项

### Requirement: 模板目录扫描

打开管理窗口和用户点击“重新扫描”时，系统必须（MUST）在后台扫描模板目录，按“路径 → inode → SHA-256（仅唯一候选）”识别条目：外部改名或移动保留 `id`、Key、使用次数；跨子目录移动视为改分组；外部删除的条目进回收站并标记“文件缺失”，文件回来后自动恢复；新文件自动导入；新子目录成为分组，超过 3 层的不导入并提示；不跟随符号链接；扫描结果按批（≤ 500 条）各自成事务，完成后显示新增/更新/缺失计数。扫描核心为纯函数。

#### Scenario: 外部改名与移动（AC-26）

- **GIVEN** 文件模板 `值班表.xlsx` 位于 `运维/`，Key 为 `zbb`，使用 5 次
- **WHEN** 用户在文件管理器中把它改名为 `排班表.xlsx` 并移动到 `行政/`，然后打开管理窗口
- **THEN** 条目仍为同一 `id`，Key `zbb` 与使用次数不变，标题变为 `排班表`，分组变为 `行政`

#### Scenario: 外部编辑后仍能识别（AC-27）

- **GIVEN** 文件模板 `上线检查单.md`
- **WHEN** 用户用编辑器以“写临时文件再改名”方式保存（inode 变化）
- **THEN** 下次扫描按相对路径识别为同一条目，更新大小、修改时间与哈希，不产生新条目

#### Scenario: 外部删除（AC-28）

- **GIVEN** 文件模板 `机房规范.pdf`
- **WHEN** 用户在外部删除该文件，然后打开管理窗口
- **THEN** 条目进入回收站并显示“文件缺失”，快搜不再返回；文件放回原位并重新扫描后条目自动恢复到原分组

### Requirement: 文件模板回收站生命周期

放入回收站时文件必须（MUST）移至 `.trash/<id>.<ext>`；还原回原分组（不存在则未分组，重名加后缀）；彻底删除、清空、自动清理时删除文件；`SearchIndex` 与回收站状态一致。

#### Scenario: 回收站中的文件（AC-29）

- **GIVEN** 文件模板 `报销单.xlsx`
- **WHEN** 用户放入回收站、还原、再放入回收站并彻底删除
- **THEN** 放入后文件位于 `.trash/<id>.xlsx`；还原后回到原分组目录；彻底删除后文件与记录都不存在

### Requirement: 先文件后数据库的补偿

所有文件写操作必须（MUST）遵守“先文件、后数据库，数据库失败时撤销文件操作”；撤销失败记录 `TEMPLATE_RECONCILE_PENDING` 并由下次扫描以磁盘为准修正；除回收站彻底删除外，任何路径都不得删除用户文件。文件写入一律“临时文件 → rename”。

#### Scenario: 导入时数据库失败

- **GIVEN** 导入文件已复制到模板目录
- **WHEN** 数据库事务失败
- **THEN** 刚复制的副本被删除，显示 `DB_WRITE_FAILED`，原文件不变

### Requirement: 文件模板可检索

检索服务必须（MUST）能按 key、aliases、title、tags 返回文件模板（不做全文检索），结果携带 `kind=file` 与 `fileExt`；管理窗口列表的文件模板行图标按扩展名、徽标为大写扩展名。对文件模板调用文本片段的复制/粘贴命令时，后端返回 `TEMPLATE_ACTION_UNSUPPORTED`，不计数、不写剪贴板。快搜窗口中文件模板的行图标、徽标、按键与底栏由 formatted-copy-export 实现（2026-10-16 人工决策 B：快搜窗口中文件模板的行图标、徽标、按键与底栏移交 SPEC-23）。

#### Scenario: 按 Key 搜到文件模板

- **GIVEN** 文件模板 Key 为 `bxd`、扩展名 xlsx
- **WHEN** 在管理窗口检索 `bxd`，以及调用快搜检索接口查询 `bxd`
- **THEN** 管理列表行显示表格图标与 `XLSX` 徽标；检索结果包含该条目且 `kind=file`、`fileExt=xlsx`

#### Scenario: 文件模板不走文本复制路径

- **GIVEN** 文件模板 `bxd`
- **WHEN** 对它调用文本片段的复制命令
- **THEN** 返回 `TEMPLATE_ACTION_UNSUPPORTED`，使用次数不变，剪贴板不被写入

### Requirement: 模板路径安全与日志隐私

模板路径必须（MUST）由 `templates/` 根加校验后的相对路径构成，规范化后仍在根内，拒绝 `..`、绝对路径与符号链接逃逸；日志只记录条目 `id`、扩展名、大小与错误码，不记录文件名、相对路径或分组名。

#### Scenario: 符号链接不被跟随

- **GIVEN** 模板目录中被放入指向 `/etc/passwd` 的符号链接
- **WHEN** 执行扫描
- **THEN** 该链接不被导入，也不读取其目标

## Non-goals and Risks

- 非目标：带格式复制、导出、快搜 `Shift+Enter`/`Ctrl+O`（formatted-copy-export）；设置中的模板目录展示（settings-backup）；内容预览、全文检索、inotify 实时监听；引用模板目录外的文件。
- 风险：文件系统与数据库非原子（由补偿与扫描收敛）；外部编辑器原子保存改变 inode（由路径优先识别缓解）；性能目标 2,000 文件无变化扫描 ≤ 2 s 需目标机实测。
