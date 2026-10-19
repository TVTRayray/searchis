# Test Report: phase10-file-templates

> Runtime 必须在归档前确认本文件包含本次 change 的实际测试命令和结果（含模板占位符时拒绝归档）。
> 自动化命令在最终提交上执行；实机项由人在归档审批前执行。

## Commands

在最终提交 `a468e35`（feat/file-templates）上执行，2026-10-19：

- `cd src-tauri && cargo test`
- `npm run build`
- `.specflow/bin/e2e-isolated.sh`（Xvfb + 私有 D-Bus + KWin 隔离显示）

## Results

- `cargo test`：exit 0；`test result: ok. 218 passed; 0 failed; 1 ignored`（lib）；main 与 doc-tests 各 0 项。4 条编译警告（未使用 import、umya 弃用方法、未使用方法 `group_name`）；sqlcipher `hmac check failed` 日志来自错误密钥用例的预期输出。被忽略的 1 项为 `qa_perf_500mb_export_and_import`（需 `--ignored`，结果见 implementer-mvej91rp-iswrx.result.yaml）。
- `npm run build`：exit 0；`✓ built in 2.45s`，仅 chunk > 500 kB 体积警告。
- `.specflow/bin/e2e-isolated.sh`：exit 0；`6 passing (21s)`——quick-search 视觉与键盘契约、SPEC-21 分组树/语言徽标、SPEC-21 编辑器历史与换行、SPEC-22 文件模板、SPEC-23 带格式复制/导出/打开、SPEC-24 设置视图与 zip 备份往返。

## 实机验收（人工，归档审批前）

> 每项记录：日期、执行人、结果（pass / fail）、证据路径。失败时对相应 spec 执行 `specflow reopen-qa`。

- [x] visual-foundation：浅/深 12 张截图（`artifacts/spec-20/`）与预览页对照；左栏底部按钮；KDE Global Menu 五组菜单
- [x] groups-language：AC-23；浅/深管理窗口截图
- [x] file-templates：AC-26（Dolphin 改名移动）、AC-27（WPS 保存后识别）、“打开方式…”列出 WPS/LibreOffice
- [x] formatted-copy-export：AC-30（WPS 文字）、AC-31（WPS 表格）、AC-32（Dolphin）、纯文本编辑器粘贴得到源码/TSV/路径；Ctrl+Enter 实机自动粘贴
- [x] settings-backup：KDE 目录对话框、Dolphin 打开模板目录、浅/深设置视图截图
- [x] 全局：`Alt+O` 呼出与自动粘贴回归、KDE Global Menu 重启后 5 s 内重注册

> 2026-10-19 实机验收中修复：文本内容编辑器圆角由胶囊（999px）改为卡片 18px（`src/index.css` `.manager-code-editor`），人工授权免二次 QA，`npm run build` 通过，待人工复核。

> 2026-10-19：人工实机验收通过（Ray 确认）。文本内容编辑器圆角修复（c9843a4）已人工复核。颗粒纹理不透明度沿用当前值（浅色 0.06、深色 0.035）。
