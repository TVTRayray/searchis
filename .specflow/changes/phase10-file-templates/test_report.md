# Test Report: phase10-file-templates

> Runtime 必须在归档前确认本文件包含本次 change 的实际测试命令和结果（含模板占位符时拒绝归档）。
> 自动化命令在最终提交上执行；实机项由人在归档审批前执行。

## Commands

- `<command>`

## Results

- `<result>`

## 实机验收（人工，归档审批前）

> 每项记录：日期、执行人、结果（pass / fail）、证据路径。失败时对相应 spec 执行 `specflow reopen-qa`。

- [ ] visual-foundation：浅/深 12 张截图（`artifacts/spec-20/`）与预览页对照；左栏底部按钮；KDE Global Menu 五组菜单
- [ ] groups-language：AC-23；浅/深管理窗口截图
- [ ] file-templates：AC-26（Dolphin 改名移动）、AC-27（WPS 保存后识别）、“打开方式…”列出 WPS/LibreOffice
- [ ] formatted-copy-export：AC-30（WPS 文字）、AC-31（WPS 表格）、AC-32（Dolphin）、纯文本编辑器粘贴得到源码/TSV/路径；Ctrl+Enter 实机自动粘贴
- [ ] settings-backup：KDE 目录对话框、Dolphin 打开模板目录、浅/深设置视图截图
- [ ] 全局：`Alt+O` 呼出与自动粘贴回归、KDE Global Menu 重启后 5 s 内重注册
