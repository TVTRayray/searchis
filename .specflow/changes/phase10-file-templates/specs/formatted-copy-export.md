# Delta Spec: formatted-copy-export

> Change: `phase10-file-templates`
> Status: `draft`
> 原 SPEC-23。详细契约（IPC 表、X11 选区适配、剪贴板格式、xlsx 转换输出、失败场景、测试点）：[`contracts/23-formatted-copy-and-export.md`](../contracts/23-formatted-copy-and-export.md)，实现与 QA 必读。预研：`docs/research/xlsx-clipboard-spike.md`。
> PRD：FR-PCK-02（v0.4 按键表与 `autoPaste` 修正）、FR-PCK-03、FR-FMT-01～04、FR-EXP-01、AC-30～35、E-25、E-26、NFR-PERF-06、D-16、D-18。

## ADDED Requirements

### Requirement: 快搜窗口按条目类型的按键语义

快搜窗口按键必须（MUST）符合 PRD FR-PCK-02 v0.4：文本片段 Enter 复制、Ctrl+Enter 粘贴、Ctrl+E 编辑；文件模板 Enter 带格式复制、Ctrl+Enter 带格式复制并自动粘贴、Shift+Enter 导出、Ctrl+O 用默认软件打开（不计数，成功后关闭）、Ctrl+E 编辑；`Ctrl+1`～`Ctrl+9` 对文件模板执行带格式复制并自动粘贴；输入法组合态下均不触发。底栏提示随选中类型切换，快捷键速查按“文本片段 / 文件模板”分组列出。

#### Scenario: 底栏随类型切换

- **GIVEN** 快搜结果中同时有文本片段与文件模板
- **WHEN** 用户在两者之间移动选中项
- **THEN** 文件模板显示“带格式复制 ↵ / 粘贴 Ctrl+↵ / 导出 Shift+↵ / 打开 Ctrl+O / 编辑 Ctrl+E / 关闭 Esc”，文本片段显示“复制 ↵ / 粘贴 Ctrl+↵ / 编辑 Ctrl+E / 关闭 Esc”

#### Scenario: 输入法组合态（AC-12 回归）

- **GIVEN** 输入法处于组合态
- **WHEN** 用户按 Enter、Shift+Enter 或 Ctrl+O
- **THEN** 不执行任何条目动作

### Requirement: autoPaste 设置生效

后端必须（MUST）以“请求标志 且 `settings.autoPaste`”作为最终自动粘贴判定，前端不再自行决定；`autoPaste=false` 时文本片段与文件模板的 Ctrl+Enter、Ctrl+数字均退化为仅复制并关闭窗口，提示“自动粘贴已在设置中关闭”。

#### Scenario: 自动粘贴开关（AC-35）

- **GIVEN** 设置中关闭自动粘贴
- **WHEN** 用户在快搜窗口对文本片段按 Ctrl+Enter
- **THEN** 只写入剪贴板，不发送 `Ctrl+V`，窗口关闭并提示自动粘贴已关闭

### Requirement: Searchis 持有 X11 剪贴板选区

带格式复制必须（MUST）由 Searchis 持有 X11 CLIPBOARD 选区：确认成为所有者后才返回成功；响应 `TARGETS`、`TIMESTAMP` 与各数据格式；超过最大请求尺寸时使用 INCR；收到 `SelectionClear` 后释放数据。无 X11 连接时 `FORMAT_COPY_FAILED`。文本片段的现有剪贴板路径不变。Searchis 退出后富文本丢失属已接受限制。

#### Scenario: 其他程序接管剪贴板

- **GIVEN** Searchis 正持有带格式复制的数据
- **WHEN** 其他程序写入剪贴板
- **THEN** Searchis 收到 SelectionClear 并释放数据

### Requirement: Markdown 带格式复制

md 模板必须（MUST）提供 `text/html`（pulldown-cmark，支持 GFM 表格、任务列表、删除线；原始 HTML 转义；链接只保留 http、https、mailto、相对路径）与 `text/plain`（文件原文）。HTML 只进入剪贴板，不在界面渲染。

#### Scenario: Markdown 带格式复制（AC-30）

- **GIVEN** md 模板内容含标题、表格和 `<script>alert(1)</script>`
- **WHEN** 用户在快搜窗口选中它并按 Enter
- **THEN** 剪贴板提供 `text/html` 与 `text/plain`；HTML 中不含 `<script>` 元素；`text/plain` 与文件逐字节一致；使用次数加 1，窗口关闭

### Requirement: xlsx 带格式复制

xlsx 模板必须（MUST）复制活动工作表已用区域（≤ 50,000 单元格，超过则 `FORMAT_COPY_TOO_LARGE` 并建议导出），提供 `XML Spreadsheet`（样式完整展开、无 `ss:Parent`、不写公式）、`text/html`（内联样式、colspan/rowspan）与 TSV；保留合并、填充、边框、字体、数字与日期格式、列宽、行高；公式写缓存值，无缓存值时写空并在结果 notes 中提示；损坏或加密文件 `FORMAT_COPY_FAILED`。格式转换为与 X11、数据库解耦的纯函数。

#### Scenario: xlsx 带格式复制（AC-31）

- **GIVEN** xlsx 模板含合并单元格、填充色、边框、千分位数字、日期和一个公式单元格
- **WHEN** 用户按 Enter 后在 WPS 表格空白工作表中粘贴
- **THEN** 合并、颜色、边框、数字格式、日期格式、列宽、行高与原文件一致；公式单元格显示计算结果而非公式

### Requirement: 其他文件类型复制

其他扩展名的文件模板必须（MUST）提供 `text/uri-list`、`x-special/gnome-copied-files`（`copy` 加 file URI）与 `text/plain`（绝对路径）。

#### Scenario: 其他文件复制（AC-32）

- **GIVEN** pdf 模板
- **WHEN** 用户按 Enter 后在 Dolphin 中粘贴
- **THEN** 目标目录出现该 pdf 的副本

### Requirement: 带格式复制计数

带格式复制必须（MUST）在成为选区所有者后于单事务中计数一次（同 `operationId` 幂等）；Enter 成功后关闭窗口；失败不计数、窗口保留并显示错误。

#### Scenario: 失败不计数

- **GIVEN** xlsx 模板超过 50,000 单元格
- **WHEN** 用户按 Enter
- **THEN** 显示 `FORMAT_COPY_TOO_LARGE` 并建议导出，窗口保留，使用次数不变

### Requirement: 导出副本

Shift+Enter 必须（MUST）把文件导出到 `exportDir`（为空时依次回退 XDG 下载目录、`~/Downloads`、`$HOME`），文件名 `标题_YYYY-MM-DD.扩展名`，重名加 ` (2)` 且不覆盖（不覆盖语义的改名，冲突递增重试）；先写临时文件再改名；成功后窗口保持打开并显示路径、计数一次；失败显示 `EXPORT_FILE_FAILED` 且无残留、不计数；导出目录不存在时不自动创建。管理窗口右栏“导出”“带格式复制”按钮启用并复用同一后端命令。

#### Scenario: 导出（AC-33）

- **GIVEN** 导出目录已有 `报销单_2026-10-15.xlsx`
- **WHEN** 用户在 2026-10-15 对模板 `报销单` 按 Shift+Enter
- **THEN** 生成 `报销单_2026-10-15 (2).xlsx`，原文件未被覆盖；快搜窗口保持打开并显示导出路径；使用次数加 1

#### Scenario: 导出失败（AC-34）

- **GIVEN** 导出目录不可写
- **WHEN** 用户导出任意文件模板
- **THEN** 显示 `EXPORT_FILE_FAILED`，导出目录中没有残留临时文件，使用次数不变

## Non-goals and Risks

- 非目标：xlsx 公式、图片、图表、批注、条件格式、数据验证；多工作表或选区选择；Searchis 退出后保留富文本；修改导出目录的界面（settings-backup）。
- 风险：X11 选区自实现（INCR、多格式）易错且不能在自动化测试中验证真实粘贴——E2E 用剪贴板替身，AC-30～32 必须实机人工验收（写真实剪贴板需人明确授权）；WPS 升级可能改变 `XML Spreadsheet` 解析，保留 HTML 与 TSV 兜底。
