# Design: phase10-file-templates

> 技术方案汇总。每个 spec 的接口、数据契约与失败场景以 contracts 目录下对应文件为准，本文件只记录跨 spec 的取舍。

## Context and Constraints

- 现有架构：React UI → Tauri IPC → Rust 应用层 → 存储/检索/平台适配层；SQLite + SQLCipher（当前 Schema v3）；X11 能力集中在 `src-tauri/src/platform/`；剪贴板写入由 `clipboard.rs` 封装；检索索引 `search_index.rs` 与回收站同步（SPEC-15 路径）。
- 必须遵守的约束（`.agent/AGENT.md`）：参数化 SQL、事务、revision 并发控制、日志不含正文/文件名/路径/查询词、敏感正文遮挡、不渲染用户 HTML、平台调用不经 shell、自动粘贴只发固定 Ctrl+V、目标环境仅 Arch + KDE Plasma 6 + X11。

## Chosen Approach

- 视觉先行：visual-foundation 只换 token、形状、字体与左栏底部按钮，后续 spec 直接在新 token 上构建，避免改两遍样式。
- Schema 分两步：v4（分组、语言、kind 预留）随 groups-language；v5（file 字段）随 file-templates。每次迁移前在同目录生成 0600 权限的 .bak，迁移单事务，失败进入只读错误页。
- 文件与数据库的一致性：固定“先文件、后数据库，失败撤销文件操作”；撤销失败记 TEMPLATE_RECONCILE_PENDING，由下一次扫描以磁盘为准收敛；任何补偿路径不删除用户文件。文件写入一律临时文件加 rename，跨文件系统退回复制、校验哈希、删除源。
- 目录扫描：纯函数核心（输入数据库快照与目录快照，输出变更计划），IO 层只收集快照、执行计划；识别顺序为路径、inode、SHA-256（仅唯一候选）；按每批不超过 500 条分事务提交；只在打开管理窗口与手动“重新扫描”时运行，不做 inotify。
- 带格式复制：格式转换为纯函数（文件字节到各格式字节），与 X11 和数据库解耦；Searchis 用 x11rb 在独立线程持有不可见窗口作为 CLIPBOARD 所有者，支持 TARGETS、TIMESTAMP、INCR、SelectionClear；文本片段的现有剪贴板路径不变。xlsx 读取用 umya-spreadsheet，主格式 XML Spreadsheet（样式完整展开、无 ss:Parent、写公式缓存值），附 HTML 与 TSV 兜底。
- autoPaste 判定移到后端：最终值等于请求标志与 settings.autoPaste 的“与”。
- 备份 v2：zip 内含 backup.json 与 templates 目录；导出与导入共用同一 schema 定义与校验函数；导入先解压到应用数据目录下的 import-staging 临时目录并校验大小与哈希，数据库单事务成功后再移入模板目录，失败用补偿事务回滚并清理临时目录。
- E2E 隔离：template_open 与剪贴板写入在 e2e-webdriver 构建中替换为记录调用参数（格式名、字节长度、哈希）的替身；自动化测试不写真实剪贴板、不启动外部应用、不触发 KDE 全局快捷键。

## Alternatives Considered

- 方案：Tauri clipboard 插件写富文本。
  - 未选择原因：不支持自定义 MIME 格式（XML Spreadsheet、x-special/gnome-copied-files）。
- 方案：一次性迁移到 Schema v5。
  - 未选择原因：groups-language 与 file-templates 分别交付、分别验收，分两步可让每个 slice 独立回滚。
- 方案：inotify 实时监听模板目录。
  - 未选择原因：增加常驻资源与事件风暴处理，扫描触发点已能满足 AC-26～28。
- 方案：沿用液态玻璃并新增主题。
  - 未选择原因：2026-10-15 需求讨论决定整体废弃玻璃（D-17）。

## Data / API / Migration Impact

- Schema v3 到 v4：groups 表；snippets.group_id、language、kind。
- Schema v4 到 v5：snippets 的 file_rel_path、file_ext、file_size、file_inode、file_sha256、file_mtime、file_original_group_id、file_auto_imported、file_missing 及其约束。
- 设置新增：openWithByExtension、exportDir。
- 备份：v1 JSON 继续可导入；导出改为 v2 zip。settings-backup 完成前导出的 v1 备份会丢失分组、语言与模板文件。
- 新增 IPC：group_*、template_*、template_format_copy、template_export、templates_dir_open；search 结果新增 language、groupName、kind、fileExt。

## Risks and Rollback

- 文件系统与数据库非原子：补偿加扫描收敛；file-templates 必须覆盖注入失败测试。
- X11 选区自实现：最小实现加单元测试；真实粘贴只能实机人工验收。
- WPS 解析变化：保留 HTML 与 TSV 兜底，预研样本留作回归用例。
- 回滚：每次 Schema 迁移保留 .bak；Phase 10 在 feat/file-templates 分支开发，settings-backup 完成并归档前不发布。
