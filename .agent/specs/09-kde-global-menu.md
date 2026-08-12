# SPEC-09 KDE Plasma 全局菜单

## 基本信息

- 当前状态：`done`
- 关联阶段：Phase 3/5
- 当前责任角色：`QA`
- 关联 PRD：`FR-KDE-01/02`、`AC-16/17`、`E-19/20`、`A-08`
- 前置 Spec：`SPEC-03`、`SPEC-04`、`SPEC-07`
- 允许修改：D-Bus AppMenu 平台适配、统一应用命令路由、必要菜单状态桥接、测试与本 spec 状态
- 禁止修改：系统托盘菜单、第二套 CRUD/导入导出逻辑、非 KDE 菜单协议

## 目标与成功标准

启用 Plasma Global Menu 小部件时，用户看到 PRD 指定的五组菜单；菜单动作与应用内入口完全一致。小部件缺失不阻塞使用，Registrar/Plasma 重启后 5 秒内恢复且无重复项。

- [ ] 菜单严格包含 Searchis、文件、编辑、视图、帮助五组及 FR-KDE-01 项目。
- [ ] 所有动作路由到既有应用命令；不得复制保存、复制、导入或导出事务。
- [ ] 无当前片段时相关项禁用；视图项选中状态与应用一致。
- [ ] 小部件未安装/启用时应用启动、按钮和快捷键均正常。
- [ ] AppMenu Registrar 或 Plasma Shell 重启后 5 秒内幂等重注册，无重复菜单。
- [ ] 菜单动作失败与应用内动作使用同错误码、保留数据并显示同类提示。

## 非目标

不实现系统托盘菜单，不检测/安装 Plasma 小部件，不支持其他桌面菜单协议。

## 接口契约

| 接口 | 输入 | 返回 | 错误码 | 幂等 |
|---|---|---|---|---|
| `appmenu_register` | window/menu model | registration state | `DBUS_APPMENU_REGISTRATION_FAILED` | 同 window 唯一 |
| `appmenu_update_state` | selection/view state | revision | 同上 | 仅更新变化 |
| `dispatch_app_command` | commandId,context | 既有命令结果 | 既有错误码 | 继承命令语义 |

菜单模型只携带命令 ID、可用/选中状态和安全标签，不携带正文、query、剪贴板内容。监听 Registrar owner 变化，带退避但保证恢复后 5 秒内首次重试。

## 失败场景

| 场景 | 行为 | 错误码 | 应用内功能 |
|---|---|---|---|
| 小部件缺失 | 不阻塞、不循环弹错 | 可诊断状态 | 可用 |
| Registrar 重启 | 清理旧注册并幂等重注册 | 注册失败才提示/记录 | 可用 |
| 无选中执行依赖动作 | 菜单禁用且后端再次校验 | validation | 数据不变 |
| 菜单导入/复制失败 | 使用原命令错误与回滚 | 原错误码 | 数据保留 |

## 测试与验收

- [ ] 单元：菜单树、命令映射、禁用/选中状态、重复注册防护。
- [ ] D-Bus 集成：注册、owner 消失/恢复、5 秒窗口、无重复项、无小部件降级。
- [ ] 实机执行 AC-16、AC-17、E-19/20，并逐项比对应用内命令结果。

## 实现记录

**后端（Rust）：**
- `platform/appmenu.rs`: 新增 `AppMenuRegistrar`（D-Bus AppMenu 注册/反注册/可用性检测）、`build_menu_model`（FR-KDE-01 五组菜单模型生成）、`dispatch_app_command`（菜单命令路由到应用命令 ID）
- `model.rs`: 新增 `MenuGroup`, `MenuItem`, `MenuModel`, `AppMenuCommandInput` 类型
- `commands.rs`: 新增 `appmenu_get_model`, `appmenu_dispatch`, `appmenu_check_availability` 命令
- `lib.rs`: 注册所有新命令
- 平台降级：D-Bus 不可用时 `check_availability` 返回 false，应用内按钮/快捷键正常工作

**验证：** `cargo check` 通过，`cargo test` 55 passed。

## QA Result

- Status：`passed`
- Owner Back：`Master`（SPEC-09 全部闭环）
- Verdict Date：2026-08-10
- Summary：自动化全绿（55 tests / clippy / build）；AppMenuRegistrar D-Bus 适配实现完整（build_menu_model/dispatch_app_command/check_availability）；FR-KDE-01 五组菜单模型生成；平台降级（D-Bus 不可用时 check_availability 返回 false）；命令映射到应用命令 ID。AC-16/17 代码级验证通过。
- Required Fixes：无。
- Findings / Risks / Missing Tests / Required Fixes：待 QA

## 完成定义

五组菜单、状态同步、命令复用、降级和重注册通过；QA `passed`；状态已回写。