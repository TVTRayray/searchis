# SPEC-06 设置、主题与开机启动

## 基本信息

- 当前状态：`todo`
- 关联阶段：Phase 4
- 当前责任角色：`Coder`
- 关联 PRD：`FR-SET-01/02`、`FR-SCH-03`、`AC-15`、`NFR 11.4`、`A-09`
- 前置 Spec：`SPEC-02`、`SPEC-03`、`SPEC-05`
- 允许修改：设置/主题前端，Settings 存储，KGlobalAccel/XDG Autostart 适配，相关测试与本 spec 状态
- 禁止修改：AppMenu、备份导入、向导、非 XDG 开机方案

## 目标与成功标准

用户可修改全局快捷键、开机启动、自动粘贴、主题、最大结果数和回收站清理周期；成功立即生效并重启保持，系统调用失败则 UI/持久化均恢复旧值。

- [ ] Settings Schema 只含 PRD 8.2 字段；`restoreClipboard=false` 且无可操作开关。
- [ ] 主题支持 light/dark/system，系统变化可跟随；减少动态效果关闭非必要动画。
- [ ] maxResultsCount 5–100 并实际影响检索；清理周期只接受 null/7/30/90。
- [ ] 快捷键复用 SPEC-03 的先注册新值再提交设置语义。
- [ ] XDG Autostart 根据真实 `.desktop` 文件及启用状态回填，不以缓存值冒充。
- [ ] 不存在音效字段、控件、依赖或资源。

## 非目标

不实现 Wayland/其他桌面开机能力，不实现数据库密钥编辑或剪贴板恢复。

## 接口契约

| 服务 | 请求 | 返回 | 错误码 | 回滚 |
|---|---|---|---|---|
| `settings_get` | 无 | Settings + launchActual | `DB_OPEN_FAILED` | 只读 |
| `settings_update` | field,value,revision | Settings | validation/平台/`DB_WRITE_FAILED` | 平台和 DB 任一步失败恢复旧值 |
| `autostart_set` | enabled | actualState | `AUTOSTART_UPDATE_FAILED` | 原子替换 `.desktop` |

外部副作用设置采用“验证 → 平台切换 → DB 提交”；DB 提交失败时补偿恢复平台旧状态，补偿失败必须明确报告双重状态并以实测值刷新 UI。

## 失败与安全

- 快捷键冲突：保留原注册和数据库值，显示原因。
- Autostart 无权限/文件异常：不显示成功，读取实际状态回填。
- Settings revision 冲突：重新加载，不静默覆盖。
- `.desktop` 内容固定生成并安全引用可执行路径，不拼接 shell 命令。
- 日志不记录具体快捷键、路径中的用户名或用户内容。

## 测试与验收

- [ ] 单元：Settings 枚举/范围/默认值和 schema migration。
- [ ] 集成：主题/maxResults 持久化；快捷键与 Autostart 成功、失败、DB 补偿失败。
- [ ] UI/a11y：三主题、系统切换、减少动态、键盘操作、无音效/restoreClipboard 控件。
- [ ] 目标机执行 AC-15、AC-11，并核对真实 XDG 文件状态。

## QA Result

- Status：`not_run`
- Owner Back：`none`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

全部设置真实生效、持久化和失败回滚通过；QA `passed`；状态已回写。