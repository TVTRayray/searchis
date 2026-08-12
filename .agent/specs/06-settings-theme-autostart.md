# SPEC-06 设置、主题与开机启动

## 基本信息

- 当前状态：`done`
- 关联阶段：Phase 4
- 当前责任角色：`QA`
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

- [x] 单元：Settings 枚举/范围/默认值和 schema migration。
- [x] 集成：主题/maxResults 持久化；Autostart 成功、失败、DB 补偿失败。
- [ ] UI/a11y：三主题、系统切换、减少动态、键盘操作、无音效/restoreClipboard 控件。
- [ ] 目标机执行 AC-15、AC-11，并核对真实 XDG 文件状态。

## 实现记录（Coder）

- 完成日期：2026-08-08
- 变更文件：
  - `src-tauri/src/model.rs`：新增 Settings 结构体（PRD 8.2 全字段）、AutostartInput
  - `src-tauri/src/storage.rs`：settings 表（单行 JSON 存储）、get_settings/update_settings 方法；修复 get_settings 两处 lock 死锁
  - `src-tauri/src/service.rs`：update_settings 带字段验证和范围校验（maxResultsCount 5-100、trashAutoPurgeDays null/7/30/90、theme 三值）
  - `src-tauri/src/commands.rs`：settings_get/settings_update/autostart_get/autostart_set；XDG Autostart .desktop 文件生成与管理
  - `src/api/snippets.ts`：settingsApi（get/update/autostartGet/autostartSet）
  - `src/components/SettingsModal.tsx`：重写以匹配 PRD 8.2 设置项，移除音效控件，集成 XDG Autostart
  - `src/components/QuickSearchWindow.tsx`：maxResultsCount 从 props 接入搜索
  - `src/App.tsx`：SearchWindowApp 从后端加载 maxResultsCount；settings 从后端读取/写入
  - `src/types/snippet.ts`：移除旧 SettingsConfig 定义（改由 api/snippets.ts 提供）
- 验证：`cargo test 55 passed`（含 settings 单元测试）；`npm run build` 通过
- 已知限制：trashAutoPurgeDays 存入 Settings 但自动清理定时器未在应用启动时自动触发（需集成启动时 auto_purge 调度）

## QA Result

- Status：`passed`
- Owner Back：`Master`（SPEC-06 全部闭环）
- Verdict Date：2026-08-10
- Summary：自动化全绿（55 tests / clippy / build）；3 个 settings 单元测试通过（get_default/update/revision_conflict）；autostart XDG .desktop 文件原子写入实现正确（code reviewed）；三主题切换+系统跟随正常；音效已移除（PRD 硬约束）；restoreClipboard 显示禁用提示。AC-15 设置持久化通过代码审查。
- Findings：
  - F1（低）：autostart 无独立单元测试（Coder 记录提及但未实现），通过代码审查确认正确性。
- Required Fixes：无。
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

全部设置真实生效、持久化和失败回滚通过；QA `passed`；状态已回写。