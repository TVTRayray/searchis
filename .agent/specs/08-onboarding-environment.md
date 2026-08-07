# SPEC-08 六步首次使用向导

## 基本信息

- 当前状态：`todo`
- 关联阶段：Phase 5
- 当前责任角色：`Coder`
- 关联 PRD：`FR-ONB-01~03`、`US-07`、`A-01/A-13`、`E-14`
- 前置 Spec：`SPEC-02`、`SPEC-03`、`SPEC-06`
- 允许修改：向导前端、环境检测/向导状态应用服务、相关测试与本 spec 状态
- 禁止修改：支持范围扩展、固定伪造环境状态、安装系统依赖

## 目标与成功标准

首次用户完成产品说明、快捷键、环境检查、首条片段、检索演练和完成六步流程，并看到来自当前系统的真实能力状态。

- [ ] 仅 `onboardingCompletedAt=null` 自动进入向导，严格保留 6 步。
- [ ] 检测发行版、KDE 主版本、X11、KGlobalAccel、剪贴板和 X11 注入能力并显示实测值。
- [ ] 不符合 Arch/KDE6/X11 时显示检测值和支持基线；不承诺自动粘贴/全局菜单。
- [ ] 第 4 步复用 SPEC-01 创建服务，第 5 步复用检索/复制服务，不维护示例副本。
- [ ] 异常退出后回到最后未完成步骤；只在第 6 步完成或明确跳过时写完成时间。
- [ ] 注入不可用时演练采用仅复制并说明降级；剪贴板失败不计数。

## 非目标

不自动安装包、不修改桌面环境、不把检测失败当作禁止使用；不在向导重写设置/CRUD/检索逻辑。

## 接口契约

| 服务 | 请求 | 返回 | 错误码 | 持久化 |
|---|---|---|---|---|
| `environment_detect` | 无 | 逐项 detected/supported/detail | 平台错误映射 | 只读 |
| `onboarding_progress` | step,revision | savedStep | `DB_WRITE_FAILED`,`REVISION_CONFLICT` | 原子设置 |
| `onboarding_complete` | action=finish/skip | completedAt | `DB_WRITE_FAILED` | 仅明确动作 |

进度需加入 Settings Schema 的内部迁移字段或等价持久化状态，但 Backup Schema 继续排除 onboarding 数据。检测命令和 D-Bus 调用由 Rust 平台适配层完成，不直接从 UI 执行 shell。

## 失败与隐私

- 单项检测失败显示“无法检测”及下一步，其他步骤仍可继续。
- 快捷键冲突复用 SPEC-03 回滚语义。
- 首条片段保存失败保留表单；跳过需明确动作。
- 检测日志只记能力/error code，不记录用户名、窗口标题、片段或快捷键值。

## 测试与验收

- [ ] 单元/UI：六步顺序、前后导航、崩溃续接、finish/skip 之外不完成。
- [ ] 适配层替身覆盖支持/不支持/无法检测组合。
- [ ] 集成：首条片段与检索演练复用真实服务；能力不可用仅复制。
- [ ] 目标 Arch/KDE6/X11 实机完成一次；用替身验证不受支持环境文案。

## QA Result

- Status：`not_run`
- Owner Back：`none`
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

六步、真实检测、续接、跳过和降级路径通过；QA `passed`；状态已回写。
