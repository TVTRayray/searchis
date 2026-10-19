# QA Report: reviewer-mveljjym-775ll

## Verdict

`pass`（上轮唯一退回项 regression 已修复；其余需求沿用 reviewer-mvelewrr-73z5d 的 pass 结论）

## Spec Coverage

- 上轮 Finding 2（regression：index.html 远程 Figtree）：
  - Evidence: `git diff index.html` 删除两条 preconnect、Figtree stylesheet 及注释，description 去掉“MacOS 风格”；`grep -rn "googleapis\|gstatic" index.html src src-tauri/src src-tauri/tauri*.json` 仅余 E2E 断言本身。E2E 新增断言（e2e/create-and-search.e2e.ts:1280-1283）读取 main 窗口 `document.documentElement.outerHTML`，断言不含 fonts.googleapis.com / fonts.gstatic.com。改动范围仅 index.html 与 e2e，符合 contracts/24 人工授权。
  - Result: pass
- 设置视图 S1、文件与导出、exportDir 校验、关于并入（AboutModal/SettingsModal 在 src、e2e 零引用）、zip 导出/导入校验、AC-36/37、重置、日志隐私、COMMIT 失败注入回滚：后端与前端逻辑自上轮无改动，cargo test 全绿，SPEC-24 E2E 用例通过。Result: pass

## Findings

- 无新增问题。上轮 `missing_test` 与 `regression` 均已解决。

## Tests Reviewed

- `cd src-tauri && cargo test` — 218 passed, 0 failed, 1 ignored（本轮复跑；sqlcipher hmac 日志为既有故障注入测试的预期输出）。
- `npm run build` — 通过（仅 chunk 体积警告）。
- `.specflow/bin/e2e-isolated.sh` — 6 passing（含 SPEC-24 与远程字体主机断言）。
- 500 MB 性能：引用 implementer 记录（release 导出 9.27 s、导入校验解压 0.49 s），未重跑。

## Design Conformance

按 docs/design/visual-qa-checklist.md 27 项逐条（证据：本轮静态 grep + cargo/build/隔离 E2E 结果；沿用 reviewer-mvelewrr-73z5d 的逐条证据，本轮仅 index.html 与 E2E 变化，未触及样式/组件）。

### 一致性（2）
1. [x] 单一 token 体系：本轮复跑 `grep -rnE "#[0-9a-fA-F]{3,8}\b|rgba?\(" src/components src/App.tsx` 无命中。
2. [x] 强调色/圆角/字体一致：SettingsView 复用 manager-* 类与 token，设置样式仅用 `var(--radius-*)`、`inherit`。

### 主题与对比度（3）
3. [ ] pending_human：明/暗设置视图截图对比（tasks 5.9）。
4. [ ] pending_human：对比度 ≥4.5:1 需实机/截图量测；设置样式只引用既有 token，未引入新色值。
5. [ ] pending_human：主题切换无白闪、重启首帧；本变更未改 ThemeProvider（localStorage 仅首帧缓存，真相源在后端）。

### 字体与圆角（3）
6. [x] 字体真实加载且 Figtree 零引用：字体经 `@font-face` 本地随包加载；`index.html` 已无 fonts.googleapis.com / fonts.gstatic.com / Figtree 引用（`grep -rn "googleapis\|gstatic" index.html src src-tauri/src` 仅余 E2E 断言），E2E 断言 main 窗口 outerHTML 不含这两个主机，隔离 E2E 6 passing。（上轮此项未通过，现已修复。）
7. [x] 圆角：`border-radius` 除 `inherit`、`var(--radius-*)` 外仅 `.quick-search-window mark` 的 3px（高亮标记，既有，非本 change）；本次无规则外圆角。
8. [x] 中文无斜体、衬线仅标题：italic 仅出现在 Playfair `@font-face` 声明，无 `font-style: italic` 使用；本次未新增衬线用法。

### 动效与无障碍（3）
9. [x] 动效有动机：进度条宽度过渡用 `--motion-manager`(220ms ≤300ms)，不确定进度动画为状态反馈；无其他新增动效。
10. [x] reduced-motion：`.settings-progress[data-indeterminate]` 在 reduced-motion 下 `animation:none`，全局规则令其余瞬时。
11. [x] 焦点可见、键盘可操作：全局 `:focus-visible`；设置控件复用 `manager-control/button`；E2E 键盘路径（Esc 返回、Tab 顺序、Enter 应用 exportDir）通过；未做完整键盘逐项审计，精细复核归入 pending_human 实机。

### 状态完整性（3）
12. [x] loading/empty/error：导入/导出 withBusy 进度遮罩；错误对话框带标题与错误码，exportDir 校验 `role="alert"`，无效包列出错误清单；设置视图无列表，empty 不适用；扫描中/文件筛选 empty 不在本 spec 范围且 SPEC-22 已验。
13. [x] 导出路径提示可见：`last-export`、默认/自定义导出目录提示、模板目录路径与“明文存放”说明（E2E 断言）；文件缺失/自动导入标记不在本 spec 范围（SPEC-22 已验）。
14. [x] 信息密度：设置视图分区卡片，无过度堆叠（E2E 通过）；精细视觉密度归入 pending_human 截图。

### 隐私与安全（视觉侧）（3）
15. [x] 敏感遮挡/编辑器不挂载：本变更未触及该逻辑，既有 E2E 用例通过。
16. [x] 文件模板无敏感开关、不预览内容：设置视图仅显示目录路径，不读文件内容。
17. [x] 日志/提示：备份模块无 log/tracing 调用；错误提示仅含错误码文本，导出成功提示中的路径为允许例外。

### 回归（2）
18. [x] 键盘语义：E2E 6 passing 覆盖创建/搜索/设置/复制导出；IME 组合态代码未改动。
19. [x] 设置持久化走后端契约：`onUpdateConfig('exportDir')` → 后端校验；SettingsView 零 localStorage 使用。

### Searchis 专项（8）
20. [x] `grep -rn "backdrop-filter" src/` 无结果；玻璃 token 与氛围光零引用。
21. [ ] pending_human：`index.css` token 与 `design-tokens.md` 逐值比对（本次 diff 仅新增 settings-* 样式、未改 token 定义；未逐值核对）。
22. [x] 无未引用组件：`src/components` 下各组件均有 import；`HeaderBar|AboutModal|SettingsModal` 在 src 零引用（本轮复跑 grep 无结果）。
23. [x] 纸张颗粒层 `pointer-events:none`（index.css 颗粒层规则）。
24. [ ] pending_human：快搜结果行 Q-A（图标/标题/`Key · 分组 · 次数`/徽标/`Ctrl+N`/高亮）——本 change 未触及快搜窗口，SPEC-23 E2E 通过，需实机确认无回归。
25. [ ] pending_human：徽标规则（文件大写扩展名、文本仅 language≠plain 显示）——本 spec 未改动，SPEC-23 E2E 覆盖视觉契约，需实机确认。
26. [x] 每界面最多一个主按钮：导入预览对话框仅 `import-confirm` 为 primary，覆盖为 danger；设置页其余为 secondary。
27. [ ] pending_human：KWin 26px 圆角与 KDE Global Menu 五组菜单（实机）。

汇总：27 项中 20 项 `[x]`，7 项 pending_human（3、4、5、21、24、25、27）；无未通过项。第 11、14 项另有实机精细复核，但已有自动化证据，判 [x]。

## Next Step

- `pass` → mark spec QA passed；pending_human 项由人在归档审批前于 test_report.md 执行。
