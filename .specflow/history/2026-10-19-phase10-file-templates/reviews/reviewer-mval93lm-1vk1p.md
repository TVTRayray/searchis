# QA Report: reviewer-mval93lm-1vk1p

## Verdict

`escalation`

- Target spec: `file-templates` / SPEC-22。
- Lease: `lease-mval93lm-tpa28`。
- Outcome: `needs_decision`；Category: `security_risk`。
- 发现下述路径安全风险，按任务要求停止后续审查，等待人工决策。测试通过不等于安全契约满足。本报告不是 QA passed 或归档批准。
- 未创建 subagent，未修改实现、tasks 或 state。

## Scope and Inputs

已读取任务包、proposal、tasks、specs/file-templates、design、ux-design、contracts/22-file-template-management、宪法及 QA 模板；核对 implementer-mvaeodqa-1ggei 与 implementer-mval622u-6arcg 的结果。

采用 2026-10-16 人工决策 B：QuickSearchWindow.tsx 的文件模板行图标、徽标、按键及底栏属于 SPEC-23，不作为本轮缺陷。kdialog/zenity 等已接受的非阻断限制不重新退回。

## Findings

### S1 — 分组目录本身的符号链接未被路径边界校验，导入可以写出模板根目录

- Severity: high / blocking。
- Category: `security_risk`。
- Expected: Delta Spec「模板路径安全与日志隐私」要求规范化后仍在 templates 根内，并拒绝符号链接逃逸；契约要求不跟随符号链接。
- Actual / source evidence:
  - `src-tauri/src/templates/paths.rs:33-55`：`safe_join(root, rel)` 从 `joined.parent()` 开始规范化，只检查祖先，未检查 `joined` 自身；`dir_join` 将非空分组目录直接交给此函数。
  - `src-tauri/src/service/templates.rs:382-393`：`template_import` 对分组调用 `dir_join` 后直接 `ensure_dir`。
  - 同文件 `import_one`（约 453-490 行）直接用 `dir_path.join(...)` 创建临时文件并 rename；最终文件路径未再经过 `safe_join`。
  - `src-tauri/src/templates/fsops.rs:25-31`：递归 create 并不拒绝已存在且指向目录的符号链接。
- 确定性触发路径（源码推导，未操作真实用户目录）：应用已运行、已有顶层分组 `行政`；将空的 `templates/行政` 目录替换为指向模板根之外某个可写目录的符号链接，然后向该分组导入文件。`safe_join(root, "行政")` 仅检查父目录 root 而返回链接路径；后续临时文件创建与 rename 跟随此目录链接，将副本写入根外。无需并发竞争，也不是导入源路径允许位于根外的问题。
- 现有 `safe_join_rejects_symlink_escape` 测试只验证 `evil/a.md`，其父目录正好是链接，不能覆盖把 `evil` 本身作为目录参数的调用。
- Impact: 违反模板写入边界，可能在管理目录外创建文件；扫描忽略链接也无法阻止这条直接导入路径。
- Requested decision: 由 runtime/人工确认并派发路径安全修复及回归测试；应覆盖「分组目录自身为根外链接」的导入/新建拒绝行为及根外目录不变。Reviewer 不在权限外修复，也不把该问题归入已接受的文件选择限制。

## Spec Coverage

- Schema v5、迁移/备份、路径/命名、纯扫描匹配、导入/新建、目录同步、回收站、补偿、搜索与不支持文本复制：针对性及全量 Rust 测试通过。具体测试运行结果见下表；不据此声称每条安全边界均已覆盖。
- 编译应用的 SPEC-22「imports, edits, opens, trashes and rescans file templates」场景通过，含管理窗口链路；基础及 SPEC-21 回归通过。
- S1 静态证据表明模板路径安全要求尚未满足；停止后未完成其余专项审查。
- 外部应用真实打开、真实文件管理器拖入、双主题视觉、完整键盘焦点循环及实机性能不由这次自动化结果替代。

## Tests Reviewed

以下均为本轮独立运行，不仅引用 implementer 声明：

| Command | Result |
| --- | --- |
| `cd src-tauri && cargo test template` | 49 passed，0 failed，87 filtered out |
| `npm run build` | 通过，TypeScript + Vite；主 JS 680.77 kB / gzip 214.52 kB |
| `.specflow/bin/e2e-isolated.sh` | 4 passing，1 spec file passed，退出码 0；包括 SPEC-22 和 SPEC-21 回归 |
| `cd src-tauri && cargo test` | 136 passed，0 failed；main/doc-tests 无测试 |

E2E 全程使用包装脚本的 Xvfb + private D-Bus + KWin，未直接运行 `npm run test:e2e`；未新增真实桌面快捷键、剪贴板、自动粘贴或外部应用输入探针。KWin 日志：`/tmp/searchis-e2e-kwin.1kjYVD.log`。未生成新的失败截图。

非失败警告：未使用的 `group_name`、Vite 大 chunk、SQLCipher wrong-key 测试的预期报错；隔离会话 portal/KCrash 输出不改变 E2E 的退出码及断言结果。

## Design Conformance

按 `docs/design/visual-qa-checklist.md` 记录；发现 S1 后停止，不将未完成项目标记通过。

### 一致性
- [ ] 全窗口 token/硬编码颜色圆角专项检查：未完成。
- [ ] 强调色、圆角、字体一致性：未完成视觉验收。

### 主题与对比度
- [ ] 明暗关键面截图：`pending_human`。
- [ ] 正文、大字、muted、选中态、主按钮对比度数值：未测。
- [ ] 切换即时生效及重启首帧：未专项验收。

### 字体与圆角
- [ ] 字体真实加载及 Figtree 零引用：构建成功不替代专项检查。
- [ ] 全应用 radius token：未完成。
- [ ] 中文无斜体/衬线标题字号：未完成。

### 动效与无障碍
- [ ] 动效动机/时长：未专项测量。
- [ ] reduced-motion：未专项验收。
- [ ] 焦点可见及完整键盘操作：`pending_human`。

### 状态完整性
- [ ] loading/empty/error：E2E 覆盖部分流程，未完成全部状态检查。
- [ ] 文件缺失/自动导入标记：Rust 与 E2E 有功能证据，未完成视觉专项；导出路径属 SPEC-23。
- [ ] 信息密度：`pending_human`。

### 隐私与安全（视觉侧）
- [ ] 敏感正文默认遮挡：基础 E2E 通过，未完成专项审查。
- [x] 文件详情不显示敏感开关、不预览内容：`FileTemplateDetail.tsx` 元数据/文件信息表单源码分支。
- [ ] 日志/提示隐私全链路：未完成；路径安全另有 S1 阻断。

### 回归
- [ ] 全键盘语义及 IME：基础 E2E 通过；真实 IME `pending_human`；SPEC-23 文件快搜行为不在范围。
- [ ] 设置持久化：未完成专项审查。

### Searchis 专项
- [ ] 玻璃/氛围光零引用：未重复专项扫描。
- [ ] token 与设计文档逐项一致：未完成。
- [ ] 组件引用及历史删除组件零引用：未完成全量枚举。
- [ ] 颗粒层不拦截拖入：真实拖入 `pending_human`。
- [ ] 快搜 Q-A：本轮文件模板展示归 SPEC-23，不作为 SPEC-22 阻断。
- [ ] 徽标规则：管理窗口 E2E 有功能证据，未完成双主题视觉确认。
- [ ] 每界面最多一个主按钮：未完成专项审查。
- [ ] KWin 实机圆角与 KDE Global Menu：`pending_human`。

## 实机验收

- `pending_human`：真实文件管理器多文件拖入、外部编辑/移动后扫描、真实 MIME 应用打开及打开方式记忆。
- `pending_human`：浅/深主题关键面、对比度、焦点循环、窗口圆角及 Global Menu。
- `pending_human`：目标机 2,000 文件扫描性能。Rust 性能测试通过不等于目标机专项测量。

## Next Step

将报告和 result 交 runtime，保持 `needs_decision` 并等待 S1 安全决策；不要据自动化测试通过推进为 QA passed。后续修复后须重新审查路径安全与尚未完成的验收项。
