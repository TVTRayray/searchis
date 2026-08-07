# SPEC-03 KGlobalAccel 呼出与 X11 自动粘贴

## 基本信息

- 当前状态：`todo`
- 关联阶段：Phase 3
- 当前责任角色：`Coder`
- 关联 PRD：`FR-ONB-02`、`FR-PCK-01/03`、`AC-03/04/11/20`、`NFR-PERF-01/03`、`E-07/08/14`
- 前置 Spec：`SPEC-02`、`SPEC-11`
- 允许修改：`src-tauri/**` 平台适配/粘贴服务，检索窗口相关前端与测试，本 spec 状态
- 禁止修改：AppMenu、XDG Autostart、向导、导入导出

## 目标与成功标准

用户在其他应用按默认 `Alt+O` 呼出 Searchis；Enter 写剪贴板后，在能力可用时激活呼出前窗口并只发送一次固定 `Ctrl+V`。

- [ ] KGlobalAccel 注册、显示/隐藏与自定义组合键替换可用；冲突时原快捷键继续有效。
- [ ] 呼出时记录 X11 目标窗口；自动粘贴前重新验证，禁止粘贴到 Searchis 自身或无效目标。
- [ ] 能力可用时完成 AC-03；不可用、运行中失效或目标关闭时完成复制并明确降级。
- [ ] 剪贴板失败绝不激活目标、不发送按键、不计使用次数。
- [ ] 同一 operationId 的剪贴板、按键发送和统计在业务层至多一次。
- [ ] 热呼出 p95≤200 ms、p99≤350 ms；≤100 KB 剪贴板写入 p95≤150 ms。

## 非目标

不支持 Wayland/其他桌面，不发送正文对应按键序列，不实现任意宏；完整设置页由 SPEC-06 承担。

## 平台接口

| 接口 | 输入 | 成功 | 错误码 | 契约 |
|---|---|---|---|---|
| `register_shortcut` | accelerator | 当前有效快捷键 | `SHORTCUT_CONFLICT`,`KGLOBALACCEL_UNAVAILABLE` | 新注册成功后才注销旧值 |
| `toggle_picker` | KGlobalAccel event | 可见状态/target token | 平台错误 | 记录呼出前 X11 窗口 |
| `execute_paste` | snippetId,operationId,autoPaste | copied/pasted/degraded | `CLIPBOARD_WRITE_FAILED`,`X11_AUTOMATION_UNAVAILABLE` | 固定 Ctrl+V，幂等 |
| `detect_capabilities` | 无 | 实测能力值 | 无 | 不使用固定“可用” |

平台适配层封装 KGlobalAccel D-Bus、X11 窗口查询/激活/键盘注入和系统剪贴板；UI 不执行 shell 命令。

## 状态顺序

捕获目标 → 写剪贴板 → 若 autoPaste 且能力可用则验证/激活目标 → 发送一次 Ctrl+V → 原子更新统计 → 按键语义关闭。目标无效或注入不可用时跳过按键但复制成功可计数。

## 失败场景

| 场景 | 行为 | 错误码 | 原设置 |
|---|---|---|---|
| 新快捷键占用/注册失败 | 保留旧注册并提示 | `SHORTCUT_CONFLICT`/`KGLOBALACCEL_UNAVAILABLE` | 保留 |
| 目标窗口关闭/切换 | 仅复制，说明目标不可用 | `X11_AUTOMATION_UNAVAILABLE` | 保留 |
| 注入能力运行中失效 | 仅复制，不假报粘贴 | `X11_AUTOMATION_UNAVAILABLE` | 保留 |
| 剪贴板失败 | 不激活、不注入、不计数 | `CLIPBOARD_WRITE_FAILED` | 保留 |

不得记录具体快捷键、窗口标题、query 或 content；目标 token 仅限进程内短期使用。

## 测试与验收

- [ ] 适配层替身：冲突回滚、D-Bus 不可用、目标失效、注入失效、重复 operationId。
- [ ] 集成：剪贴板必须先于激活/注入；仅发送固定 Ctrl+V；失败不计数。
- [ ] 实机执行 AC-03、04、11、20 和 E-07/08/14。
- [ ] 性能至少 100 次，记录冷热状态、p95/p99 和环境。

## QA Result

- Status：`not_run`
- Owner Back：`none`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

成功、降级、冲突回滚、幂等与性能均有证据；QA `passed`；状态已回写。