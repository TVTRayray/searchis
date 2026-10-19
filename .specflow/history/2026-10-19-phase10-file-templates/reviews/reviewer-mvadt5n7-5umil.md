# QA Report: reviewer-mvadt5n7-5umil

## Verdict

`repair`

- Target spec: `groups-language` / SPEC-21。
- Task lease: `lease-mvadt5n7-qg5ds`。
- Category: `implementation_defect`。
- 审阅已完成，但本 spec 不可判定通过：两处 CodeMirror 集成缺陷已在编译后的应用中复现。无需更改需求或架构；仅退回下列修复项。
- 未修改实现、规划工件或 state.yaml，未启动 subagent，未执行批准/归档/QA 状态推进。

## Spec Coverage

| Requirement / scenario | Evidence | Result |
| --- | --- | --- |
| Schema v4、默认值、0600 迁移备份、失败回滚 | `migrates_v3_to_v4_with_defaults_and_backup`、`failed_v4_migration_rolls_back_and_keeps_backup`，全量 Rust 测试通过 | 自动化覆盖通过；启动只读错误页未单独注入 UI 验证 |
| 分组名称、同级大小写唯一、深度、成环、revision、requestId 幂等 | `validation::tests::group_name_rules` 及 `storage::tests::group_*` | 通过 |
| 非空分组删除、子项上移、冲突后缀、事务回滚 | `group_delete_moves_contents_up_and_suffixes_conflicts`、`failed_group_delete_rolls_back_everything`；SPEC-21 E2E | 通过 |
| 左栏结构、分组树、未分组、计数、标签折叠 | `GroupTree.tsx`、`ManagerWindow.tsx`；SPEC-21 E2E 分组创建/删除 | 主要流程通过；完整视觉验收见下 |
| language 白名单、保存分组存在性 | `language_whitelist_and_default`、`snippet_save_rejects_missing_group_and_counts_ignore_trash` | 通过 |
| CodeMirror、语言徽标、按需语言包 | `CodeEditor.tsx`；build 分离 shell/toml/properties/sql/yaml/json chunks；SPEC-21 E2E BASH 徽标 | 基础通过；正文保真及撤销失败，见 R1/R2 |
| 语言仅影响显示、正文逐字节一致 | reviewer 编译应用探针：只打开 CRLF 正文并保存，持久化结果变为 LF | **失败 R1** |
| Ctrl+Z 正常、片段间编辑历史隔离 | reviewer 编译应用探针：选 A 再选 B，B 内 Ctrl+Z 得到 A 正文 | **失败 R2** |
| 类型筛选、文件为空 | SPEC-21 E2E 文件筛选空状态及返回文本列表 | 通过 |
| Q-A、分组名不参与检索、重命名刷新副标题 | `search_results_carry_language_group_name_and_kind`、两项现有 E2E、源码审阅 | 通过主要自动化路径 |
| 敏感未展示时不挂载正文 | ManagerWindow 的遮挡分支与 CodeEditor 条件挂载；基础 E2E 敏感流程 | 本次审阅未发现该分支新增问题 |
| 分组日志隐私 | commands/service 错误映射与日志调用源码审阅；实现者注明未加专门日志测试 | 静态检查未见分组名写日志；未声称实测日志场景通过 |
| 10k 条目性能 | `qa_perf_search_10000_snippets_p95_p99`，81 项 Rust 测试之一 | 通过 |
| AC-23 中文 IME、真实复制/粘贴逐字节、双主题设备截图 | 未操作真实桌面 | `pending_human`；先修复 R1/R2 |

## Findings

### R1 — 打开 CRLF 正文再保存会无意改写换行

- Severity: **P1**（无正文编辑也会改写持久化内容）。
- Category: `implementation_defect`
- Expected: 语言/编辑器仅影响显示；打开片段、修改元数据或直接保存时，正文原始字节保持不变。
- Actual: `line1\r\nline2\r\n` 保存后成为 `line1\nline2\n`。
- Root cause / location: `src/components/ui/CodeEditor.tsx:89-110,137-145`。EditorState 未配置换行保真；`doc.toString()` 默认返回 LF。外部 value 同步 effect 比较原始 CRLF 与标准化文档，触发整段替换；updateListener 把 LF 回写表单状态。
- Reproduction:
  1. 用合法 IPC 创建 `language=plain`、正文为 `line1\r\nline2\r\n` 的片段。
  2. 刷新管理窗口，选中片段，不编辑正文，点击“保存片段数据”。
  3. 通过 `snippet_list` 比较保存前后正文。
- Compiled-app evidence (`reviewer-mvadt5n7-5umil.probe.e2e.ts`):

  ```text
  REVIEW_CRLF_OBSERVED {"before":"line1\r\nline2\r\n","after":"line1\nline2\n"}
  ```

- Required repair: 保留未编辑正文的原始换行/字节，避免初始化或语言/元数据变化触发正文标准化回写；明确处理编辑后的换行序列，不仅修显示。补充 CRLF 打开后直接保存、只改语言/分组、LF/CRLF 与末尾换行的回归测试。不得用会继续丢失原始字节的统一 LF 转换规避契约。

### R2 — 切换片段后 Ctrl+Z 可撤销回上一条正文

- Severity: **P1**（可把其他片段正文误保存到当前片段）。
- Category: `implementation_defect`
- Expected: 选择 B 时，其编辑器初始内容为 B，撤销历史不包含 A；未编辑前 Ctrl+Z 不应替换成 A。
- Actual: A 正文 `AAA`、B 正文 `BBB`；依次选择 A、B，再在 B 正文内 Ctrl+Z，选中项仍为 B，编辑区变为 `AAA`。
- Root cause / location: `src/components/ManagerWindow.tsx:214-236,546-556,678-682` 与 `src/components/ui/CodeEditor.tsx:84-96,137-145`。选中 id 改变后，父组件在 effect 中才重置 contentInput；带新 key 的 CodeEditor 首次挂载仍收到上一个片段的值。随后同步 B 正文的普通 dispatch 被 history 收录，Ctrl+Z 可回到 A。仅更换 key 未隔离这次初始化写入。
- Reproduction: 建立 A/AAA 和 B/BBB → 选 A → 选 B → 聚焦 B 编辑区 → Ctrl+Z。
- Compiled-app evidence:

  ```text
  REVIEW_UNDO_OBSERVED {"body":"AAA","selected":"review-probe-breview-probe-b更新于 10/16/2026"}
  ```

- Required repair: 保证切换文档时编辑器以对应文档初始化且清空跨文档历史；初始化/外部加载不应作为用户编辑加入历史。补充 A→B、首次打开、普通输入撤销/重做及新建路径测试，确认撤销永远不会带回别的片段正文。

## Tests Reviewed

本次独立执行（不是仅引用 implementer 的结果）：

| Command / check | Result |
| --- | --- |
| `cd src-tauri && cargo test group_` | 11 passed，0 failed |
| `cd src-tauri && cargo test` | 81 passed，0 failed；有未使用 `group_name` 警告；wrong-key 测试的 SQLCipher 错误输出不影响结果 |
| `npm run build` | 通过；主 JS 656.25 kB / gzip 208.36 kB，Vite >500kB 提示；语言包独立 chunk |
| `npm run test:e2e`，在下述 Xvfb + 私有 D-Bus + KWin 环境中执行 | **2 passing**，包含 SPEC-21 与原有快搜/键盘契约 |
| reviewer 独立编译应用缺陷探针 | **1 passing（表示两处错误行为均被观察到，不是验收通过）**；真实 IPC 读回 CRLF 变化，DOM 键盘事件观察跨片段撤销 |
| Node CodeMirror 状态最小复现 | 确认默认 doc.toString 标准化 CRLF，外部整段替换进入撤销历史 |
| 组件颜色/圆角、玻璃/Figtree 静态 grep | 下述查询均无命中 |

E2E 隔离命令（不操作真实前台窗口、真实 KDE 全局快捷键或用户真实剪贴板；测试程序另有临时 XDG data/config）：

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' dbus-run-session -- bash -c '
  kwin_x11 --replace >/tmp/searchis-reviewer-mvadt5n7-kwin.log 2>&1 & wm=$!
  trap "kill $wm 2>/dev/null || true" EXIT
  npm run test:e2e
'
```

探针使用同样的 Xvfb/私有 D-Bus/KWin；临时目录作为 `XDG_DATA_HOME` / `XDG_CONFIG_HOME`，设置 `SEARCHIS_E2E=1` 后运行：

```sh
node node_modules/@wdio/cli/bin/wdio.js run wdio.conf.ts \
  --spec .specflow/changes/phase10-file-templates/reviews/reviewer-mvadt5n7-5umil.probe.e2e.ts
```

最终日志：`/tmp/searchis-reviewer-mvadt5n7-e2e-kwin.log`、`/tmp/searchis-reviewer-mvadt5n7-probe.log`；关键缺陷输出已在本报告保留，避免依赖临时日志。

环境重试说明：首次仅用 Xvfb 无窗口管理器，基础 E2E 因 `xdotool windowactivate` / `_NET_ACTIVE_WINDOW` 失败（SPEC-21 当时仍通过）；第一次补 KWin 使用了该版本不支持的参数。去掉不支持参数后最终两项全部通过，不作为产品回归。探针初版 requestId 非 UUID，被契约正确拒绝，修正为 `crypto.randomUUID()` 后复现成功。私有 D-Bus portal/systemd 告警不改变最终测试退出码。

已查看环境失败截图：`artifacts/e2e/1792119946991-keeps-the-quick-search-visual-and-keyboard-contract-on-real-data.png`（浅色管理窗口初始空状态）；它不构成双主题完整视觉验收。

## Design Conformance

按 `docs/design/visual-qa-checklist.md` 逐项记录；`[ ]` 表示未完成验收，不能解释为通过。范围仅 SPEC-21 新增/修改表面；SPEC-22 以后内容不提前验收。

### 一致性

- [x] 同一 token 体系：`grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components src/App.tsx` 无命中。
- [x] 新增组件强调色、圆角与字体均引用已有 token；对新增组件执行 `borderRadius:|border-radius:|rounded-\[` 搜索无命中；新增 CSS 使用 radius-pill/row 和既有字体变量。

### 主题与对比度

- [ ] 明/暗双模关键面截图对比：`pending_human`，沿用 tasks 2.10；本次仅检查失败浅色截图，不宣称完成。
- [ ] 正文/大字/muted/选中/主按钮数值对比度：未逐项测量，待双主题视觉验收补充。
- [x] 既有 E2E 主题切换契约通过；新增 CodeMirror 配色使用 CSS 变量。
- [ ] 重启首帧无白闪/跳变：未专项测量，`pending_human`。

### 字体与圆角

- [x] 构建输出本地字体资源，既有 E2E 字体检查通过；`Figtree` 静态检索无命中。
- [x] 本次新增 CSS 圆角只引用已声明 token；未改 token 值。
- [x] CodeMirror comment 显式 normal；新增分组/语言界面未引入中文斜体或小号衬线标题。

### 动效与无障碍

- [x] 本次新增 CSS 未引入新动画/transition，无新增超时动效。
- [x] 既有 reduced-motion 规则未改，基础 E2E 回归通过。
- [ ] 完整键盘流程不可判为通过：Ctrl+Z 跨片段回退失败（R2）；新对话框完整焦点循环未专项验收。

### 状态完整性

- [x] SPEC-21 空分组/文件筛选空状态与分组错误显示有实现；自动化覆盖深度超限、删除非空分组与文件筛选空状态。扫描/导入 loading 不属本 slice。
- [x] 文件缺失、自动导入、导出路径提示：本 spec 不适用，留 SPEC-22/23/24。
- [ ] 信息密度完整评估：浅色空状态已检查，非空双主题表面仍待视觉验收。

### 隐私与安全（视觉侧）

- [x] 敏感默认遮挡分支未展示时不渲染 CodeEditor 正文；基础敏感流程 E2E 通过。
- [x] 文件模板敏感开关/文件预览：本 spec 不适用。
- [x] 分组日志源码检查未见名称写入；本次 QA 探针仅使用合成正文；未执行真实用户内容日志验证。

### 回归

- [x] 既有快搜键盘/IME 组合态 E2E 通过；这不等于编辑器 IME 与撤销验收通过（R2）。
- [x] 主题/设置仍使用既有后端 API，未新增 localStorage 真相源。

### Searchis 专项

- [x] `grep -rnE 'backdrop-filter|Figtree|glass|ambient' src/` 无命中。
- [x] `src/index.css` 差异仅新增 SPEC-21 样式，未改既有 token 定义；未重复做 SPEC-20 全量 token 验收。
- [x] 新增 GroupTree、GroupDialog、CodeEditor 均由 ManagerWindow import；未新增死组件。全仓既有组件清单未重新枚举。
- [x] 纸张颗粒层既有实现未改，基础 E2E 回归通过。
- [x] Q-A 结构与徽标条件源码符合：类型图标、标题、Key/分组/次数、徽标、Ctrl+N、高亮；现有 E2E 覆盖主要结构。
- [x] 文本语言非 plain 时显示徽标；文件扩展名规则本 spec 不适用。
- [x] 新增分组对话框仅一个 primary 提交按钮；既有保存按钮主次逻辑未增设多主按钮。
- [ ] KWin 合成器实机 26px 圆角、KDE Global Menu 五组菜单：`pending_human`，虚拟 Xvfb 验证不替代实机。

## 实机验收

- `pending_human`：AC-23 中文输入法组合、Tab、Ctrl+Z/Ctrl+Shift+Z（先修 R2）、真实复制/粘贴字节保真（先修 R1）、双主题管理/快搜/分组对话框/高亮截图。
- `pending_human`：真实 KWin 圆角与 KDE Global Menu。
- 本次未自动化真实全局快捷键、真实前台输入或真实剪贴板；未勾选 tasks 2.10。

## Next Step

`repair` → runtime 仅将 R1/R2 及相应回归测试退回 Implementer。保留既有功能与 token，不扩展到文件模板、平台层或备份格式。修复后重新运行针对性编辑器回归、Rust、build、既有 E2E，再交 Reviewer；实机项仍按 `pending_human` 跟踪。本报告不推进工作流状态。
