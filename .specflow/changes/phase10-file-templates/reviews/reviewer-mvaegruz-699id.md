# QA Report: reviewer-mvaegruz-699id

## Verdict

`pass`

- Target spec: `groups-language` / SPEC-21。
- Lease: `lease-mvaegruz-lpaif`。
- 复审 implementer-mvaedw2a-me3te 对上一轮 R1/R2 的修复，未发现需退回的新缺陷。实机验收仍为 `pending_human`，本结论不是归档批准。
- 未修改实现、tasks 或 state，未创建 subagent；状态推进交 runtime。

## Spec Coverage

- 输入：任务包、proposal、tasks、specs/groups-language、design、ux-design、contracts/21-groups-and-language，以及上一轮 QA 与本轮 implementer result。
- 分组树、校验、深度/成环、删除内容上移与回滚、revision/幂等、v3→v4 迁移及备份、语言白名单与搜索 DTO：独立 Rust 全量测试通过；编译应用 SPEC-21 E2E 通过。
- 分组/语言 UI 与快搜徽标：既有 SPEC-21 E2E 通过，未扩大到文件模板后续 slice。
- R1 正文保真：CodeEditor 按初始正文配置 `EditorState.lineSeparator`，读取及受控值比较使用 `state.sliceDoc()`；CRLF 打开后直接保存，E2E 经真实 IPC 读回原正文完全一致。
- R2 文档撤销隔离：ManagerWindow 在装载不同文档或新建时批量更新正文与 editorSeq；CodeEditor 以该序号重挂载，外部值同步标注 `Transaction.addToHistory.of(false)`。A→B 后 Ctrl+Z 的编译应用回归通过。
- 普通撤销/重做：Node 使用当前安装的 CodeMirror 状态与 history API，分别验证 LF、CRLF、CR、混合换行正文的初始往返、插入、undo、redo，4 组全部通过。此检查不替代实机 IME。
- 性能：全量测试包括 `qa_perf_search_10000_snippets_p95_p99` 通过；本次没有单独采集 100 分组场景 p95/p99，不据此宣称完成全部性能专项。

## Findings

无需退回的实现缺陷；上一轮 R1/R2 已关闭。

非阻断限制：真实中文 IME、真实复制/粘贴保真、完整键盘/对话框焦点循环、双主题视觉及实机窗口菜单行为仍需人工补齐。具体见下文；不自动勾选 tasks 2.10。

## Tests Reviewed

本次独立执行：

| Command / check | Result |
| --- | --- |
| `cd src-tauri && cargo test groups_` | 0 tests，过滤器未匹配，不作为覆盖证据 |
| `cd src-tauri && cargo test language_` | 2 passed |
| `cd src-tauri && cargo test migration_` | 2 passed |
| `cd src-tauri && cargo test` | 81 passed，0 failed；包含完整分组、迁移、语言测试 |
| `npm run build` | 通过；主 JS 656.51 kB / gzip 208.38 kB；语言包独立 chunk |
| `npm run test:e2e` | 3 passing，含新增 R1/R2 回归 |
| 下述隔离环境再次运行 `npm run test:e2e` | 3 passing，1 spec file passed，退出码 0 |
| Node CodeMirror 状态探针 | 4 组换行保真及普通 undo/redo 通过 |

最终隔离验收命令：

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' dbus-run-session -- bash -c '
  kwin_x11 --replace >/tmp/searchis-reviewer-mvaegruz-kwin.log 2>&1 & wm=$!
  trap "kill $wm 2>/dev/null || true" EXIT
  npm run test:e2e
'
```

测试环境说明：首次直接运行现有 `npm run test:e2e`，继承 `DISPLAY=:0`，数据/config 虽由脚本隔离，但显示未隔离；随后完整改用上述 Xvfb/私有 D-Bus/KWin 重跑。首次运行不得描述为显示隔离或实机人工验收。后续应始终使用隔离显示，避免现有套件的窗口/键盘操作影响当前桌面。本次没有另行编写真实全局快捷键、自动粘贴或用户剪贴板探针。

警告：既有未使用 group_name、Vite >500kB、Linux 包名诊断告警；wrong-key 测试产生预期 SQLCipher 错误。隔离会话结束有 portal/KCrash 输出，但 E2E 三项通过且命令退出码为 0，不判产品失败。没有新增失败截图，未执行双主题截图验收。

## Design Conformance

依据 `docs/design/visual-qa-checklist.md`，范围仅 SPEC-21；前轮静态证据参见 reviewer-mvadt5n7-5umil.md。本轮逐项复核如下，未测量项不标通过。

### 一致性
- 同一 token：通过。CodeEditor 高亮、光标、背景、字体仍引用既有 CSS 变量。
- 强调色/圆角/字体统一：通过。对 CodeEditor、GroupTree、GroupDialog、ManagerWindow 运行颜色和圆角硬编码 grep 无命中。

### 主题与对比度
- 明暗关键面截图：`pending_human`。
- 正文、大字、muted、选中、主按钮对比度：未逐项数值测量，待人工。
- 系统跟随/主题切换：既有 E2E 契约通过；修复未改主题机制。
- 重启首帧无闪烁：未专项测量，`pending_human`。

### 字体与圆角
- 目标字体与本地资源：构建和既有 E2E 通过；本轮未新增字体。
- radius token：通过静态检查，无新增硬编码。
- 中文无斜体/小号衬线：新增界面无此样式，CodeMirror comment 保持 normal。

### 动效与无障碍
- 动效时长/easing：修复未新增动效。
- reduced-motion：既有规则未改，基础 E2E 通过，未单独完成全部动效测量。
- 键盘/焦点：R2 自动回归通过；普通 undo/redo 状态测试通过；完整焦点循环、真实 IME/键盘仍 `pending_human`。

### 状态完整性
- 分组空态、错误、删除确认及文件筛选空态：已有实现与 E2E 证据，回归通过；扫描/导入 loading 不属此 slice。
- 文件缺失/导出路径：不适用，留后续 spec。
- 双主题信息密度完整评估：`pending_human`。

### 隐私与安全（视觉侧）
- 敏感未展示时不挂载 CodeEditor：源码分支保持，基础敏感流程回归通过。
- 文件敏感开关/文件预览：不适用。
- 日志正文泄漏：修复未新增日志；测试仅使用合成正文，不宣称完成真实用户数据日志审计。

### 回归
- 快搜键盘/组合态：既有 E2E 通过，不替代编辑器真实 IME 验收。
- 主题/设置持久化：修复未改后端 API 或新增 localStorage 真相源。

### Searchis 专项
- 玻璃/氛围光/Figtree：`grep -rnE 'backdrop-filter|Figtree|glass|ambient' src/` 无命中。
- SPEC-20 token：未修改，未重复全量视觉验收。
- 新增组件有调用入口：CodeEditor/GroupTree/GroupDialog 由 ManagerWindow 使用，未全仓枚举历史死组件。
- 颗粒层 pointer-events：既有规则未改，基础 E2E 回归通过。
- Q-A 类型图标/标题/Key/分组/次数/徽标/快捷键/高亮：现有源码与基础 E2E 证据维持。
- 徽标条件：非 plain 文本显示语言，SPEC-21 E2E 通过；文件扩展名不适用。
- 每界面一个主按钮：修复未改变按钮结构，沿用上一轮静态证据。
- KWin 实机圆角及 KDE Global Menu：`pending_human`，Xvfb 不替代实机。

## 实机验收

- `pending_human`：AC-23 中文 IME、Tab、真实 Ctrl+Z/Ctrl+Shift+Z、真实复制粘贴字节保真。
- `pending_human`：浅/深主题管理窗口、分组对话框、快搜与语法高亮截图、数值对比度及完整焦点循环。
- `pending_human`：真实 KWin 合成圆角、Global Menu、重启首帧。

## Next Step

将本报告及 result 交 runtime 导入；Reviewer 不自行标记 QA passed、不批准、不归档。人工验收在归档审批前补入 test_report.md；如失败由 runtime reopen-qa。
