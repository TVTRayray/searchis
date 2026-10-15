# xlsx 带格式复制预研（2026-10-15）

目标：Searchis 把 xlsx 模板写入剪贴板，WPS 表格（Linux，X11）粘贴时尽量完整还原。

## WPS 复制时提供的剪贴板格式

`XML Spreadsheet`、`text/html`、`text/plain`/`UTF8_STRING`（TSV）、RTF、`image/bmp`、
`WPS Spreadsheets 6.0 Format`（私有二进制）、`Kingsoft Data Descriptor`、`Kingsoft Shapes Tag`。

## 结论

- **主力格式：`XML Spreadsheet`（SpreadsheetML 2003，公开格式）。** 由外部程序单独提供该格式时，WPS 能够接受。
- 已验证可还原：横向/纵向合并（`ss:MergeAcross` / `ss:MergeDown`）、字体（字号、粗体、斜体、颜色）、填充色、
  边框（含粗细）、对齐、千分位等数字格式、日期格式、列宽、行高。
- **样式不能使用 `ss:Parent` 继承**：WPS 会把父样式解析错（单元格被套上无关样式）。每个样式都要完整展开写出，WPS 自己的输出也是这样。
- **公式只认 R1C1 写法**（`=RC[-2]*RC[-1]`、`=SUM(R[-4]C:R[-1]C)`）；A1 写法（`=SUM(C3:C6)`）会被丢弃并变成数值。
- **公式粘贴后不会正确计算**：只有部分单元格能算对，其余显示 `#VALUE!` 或 0；按 F9 无效，点进编辑栏回车后才正确。
  WPS 自己生成的 SpreadsheetML 与我们的写法一致，WPS 内部复制粘贴能成功是走了私有格式。
  “单独提供 WPS 原样输出”的对照试验未完成，问题已暂时搁置。
- Ctrl+Alt+F9（全量重算）在 Linux 上会切换 TTY，不可用。

## 对 V1 的决定

- xlsx 带格式复制 = 格式 + 值：公式单元格写入计算结果，不写 `ss:Formula`。需要公式的场景用导出文件（Shift+Enter）。
- 剪贴板同时提供 `XML Spreadsheet` + `text/html` + `text/plain`（TSV）。
- 范围：单个工作表的已用区域；图片、图表不支持。
- 需要 Searchis 自己持有 X11 剪贴板并提供自定义格式名（Tauri clipboard 插件不支持）；Searchis 退出后只剩剪贴板管理器保留的纯文本。

测试样本：本次会话 scratchpad 中的 `test.xml`（含 `ss:Parent`）、`test2.xml`（无继承 + 公式变体，缓存值 -1）、`wps-formula.xml`（WPS 原样输出）。
