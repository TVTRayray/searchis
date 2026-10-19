# Delta Spec: settings-backup

> Change: `phase10-file-templates`
> Status: `draft`
> 原 SPEC-24。详细契约（IPC 表、zip 结构与校验、导入临时目录、失败场景、测试点）：[`contracts/24-settings-view-and-zip-backup.md`](../contracts/24-settings-view-and-zip-backup.md)，实现与 QA 必读。
> PRD：FR-SET-01（v0.4 设置视图与“文件与导出”）、FR-DAT-01～03（v0.4）、8.2、8.3 `SearchisBackupV2`、AC-36、AC-37、A-14、D-12。

## ADDED Requirements

### Requirement: 设置视图 S1

设置必须（MUST）从弹窗改为管理窗口内的设置视图：左栏与管理窗口左栏同宽同样式，分节导航为“返回片段、快捷键与启动、粘贴行为、外观主题、文件与导出、检索与数据、备份与重置、关于”；点击跳转对应分节；“返回片段”与 Esc 回到管理视图。

#### Scenario: 分节导航与返回

- **GIVEN** 设置视图已打开
- **WHEN** 用户点击“文件与导出”，再按 Esc
- **THEN** 先滚动到“文件与导出”分节，然后回到管理视图

### Requirement: 文件与导出设置

“文件与导出”分节必须（MUST）显示当前导出目录（默认显示解析后的 XDG 下载目录）及“选择…”（系统目录对话框，不可用时允许手动输入）与“恢复默认”；显示文件名规则说明；显示只读模板目录路径与“打开”按钮；说明“文件模板以明文存放，不受数据库加密保护”。选择的目录写入 `settings.exportDir` 即时生效，不存在或不可写时以 `VALIDATION_FAILED` 拒绝保存。

#### Scenario: 修改导出目录后导出

- **GIVEN** 用户把导出目录改为可写目录 `~/模板导出`
- **WHEN** 在快搜窗口对文件模板按 Shift+Enter
- **THEN** 文件写入 `~/模板导出`

#### Scenario: 不可写目录被拒绝

- **GIVEN** 所选目录不可写
- **WHEN** 用户确认选择
- **THEN** 显示 `VALIDATION_FAILED`，`exportDir` 保持原值

### Requirement: 关于并入设置

“关于”必须（MUST）成为设置视图的一个分节，内容与原 AboutModal 一致；AboutModal 删除且零引用；KDE Global Menu“关于”打开该分节，命令 ID 不变。

#### Scenario: 从全局菜单打开关于

- **GIVEN** KDE Global Menu 可用
- **WHEN** 用户点击菜单中的“关于”
- **THEN** 管理窗口打开设置视图并定位到“关于”分节

### Requirement: zip 备份导出

备份导出必须（MUST）生成 `searchis-backup-YYYYMMDD-HHmmss.zip`，包含 `backup.json`（`schemaVersion: 2`，含 `groups`、`snippets`、`settings`）与 `templates/`（含 `.trash/`）；`backup.json` ≤ 100 MB、zip ≤ 1 GB，超限拒绝并显示估算大小；含敏感片段时二次确认；说明“备份包含文件模板原文件”；先写临时文件再改名，目标存在时不覆盖。

#### Scenario: 含敏感片段的导出

- **GIVEN** 库中有敏感片段
- **WHEN** 用户导出备份
- **THEN** 先出现二次确认，确认后生成 zip，取消则不生成任何文件

### Requirement: zip 备份导入与校验

导入必须（MUST）接受 v2 zip 与 v1 JSON（v1 条目全部视为未分组文本片段）；zip 校验拒绝绝对路径、`..`、符号链接、未被引用的文件、哈希不符的文件，以 `BACKUP_ARCHIVE_INVALID` 报错；解压防护为单文件 ≤ 20 MB、总量 ≤ 1 GB、条目 ≤ 20,000。文件先解压到临时目录并校验大小与 SHA-256，数据库单事务成功后再移入模板目录；任一步失败清理临时目录，数据库与模板目录保持原状；启动时清理遗留临时目录。冲突策略沿用 FR-DAT-02，文件模板覆盖时替换文件，分组按 ID 合并、同级重名加 `(2)`。导入期间禁用管理窗口写操作并暂停扫描。

#### Scenario: 恶意备份包（AC-37）

- **GIVEN** zip 备份包含 `../evil.sh` 或绝对路径条目
- **WHEN** 用户导入
- **THEN** 校验阶段拒绝并显示 `BACKUP_ARCHIVE_INVALID`，模板目录之外没有任何文件被写入

#### Scenario: zip 备份往返（AC-36）

- **GIVEN** 片段库含 2 层分组、文本片段、md 与 xlsx 模板以及回收站中的模板
- **WHEN** 用户导出备份，执行“重置示例数据”，再导入该备份
- **THEN** 分组、条目元数据与模板文件（含回收站中的）全部恢复，文件 SHA-256 与导出前一致

### Requirement: 重置示例数据清理模板

重置示例数据的确认文案必须（MUST）注明会删除文件模板；模板目录先改名为临时目录，数据库事务成功后再删除，失败时改回。

#### Scenario: 重置失败恢复

- **GIVEN** 存在文件模板
- **WHEN** 重置时数据库事务失败
- **THEN** 模板目录改回原名，文件与数据库均保持重置前状态

### Requirement: 备份日志隐私

日志必须（MUST）只记录备份相关的数量、大小与错误码，不记录备份路径与文件名。

#### Scenario: 导入失败日志

- **GIVEN** 导入因哈希不符失败
- **WHEN** 查看诊断日志
- **THEN** 只有错误码与计数，无文件名或路径

## Non-goals and Risks

- 非目标：备份加密、增量或定时备份；数据目录、日志目录展示维持现状。
- 风险：导入事务成功但移入模板目录失败需补偿事务回滚数据库；500 MB 级模板导出导入不设硬指标，但界面不得冻结并显示进度。
