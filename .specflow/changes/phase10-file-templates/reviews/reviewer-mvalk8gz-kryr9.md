# QA Report: reviewer-mvalk8gz-kryr9

## Verdict

`escalation`

- Target: `file-templates` / SPEC-22；lease: `lease-mvalk8gz-sm3xk`。
- Outcome: `needs_decision`；Category: `security_risk`。
- 本次以完整复审为目标，但复核实现者明确披露的目录级 TOCTOU 时发现尚未满足的安全边界。遵守任务“遇到架构/安全风险写 needs_decision 并停止”，未继续后续专项或测试。本报告不是完整复审通过，也不批准、归档或推进状态。
- 沿用人工决策 B：QuickSearchWindow.tsx 的文件模板展示属 SPEC-23；kdialog/zenity 等已接受限制不退回，不作为本次升级原因。
- 无 subagent；未修改源码、tasks、state；仅填写本任务 report/result。

## Spec Coverage

- 上轮 `reviews/reviewer-mval93lm-1vk1p.md` S1 的原始、无需竞争的分组目录链接绕过：**源码层面已修复**。`paths.rs:33-75` 的 `verify_under` 逐段检查至目标自身，`safe_join/dir_join` 复用它；`template_import` 使用 `ensure_dir_in`，`import_one` 在创建临时文件前再次校验。新增 `safe_join_rejects_leaf_and_intermediate_symlinks` 覆盖目录自身、嵌套及根内链接。未独立运行该测试，不能据此声称运行通过。
- 路径安全“规范化后仍位于 templates 根内、拒绝符号链接逃逸”：**仍有下述安全风险，需决策**，不将 S1 整体标记关闭。
- 导入、新建、分组同步、扫描、回收站、搜索、迁移及全部失败路径的完整复审：安全停止后未完成；历史通过不是本轮通过证据。

## Findings

### S1-R — 静态目录链接已拒绝，目录级 TOCTOU 仍允许写出模板根

- Severity: high / blocking。
- Category: `security_risk`。
- Expected: 文件模板的目标写入必须留在模板根内。只有叶子 `O_NOFOLLOW` 不能保证这一目录边界。
- Actual:
  - `src-tauri/src/templates/fsops.rs:34-38`：`ensure_dir_in` 是独立的“校验→创建→校验”，不是绑定目录句柄的操作。
  - `src-tauri/src/service/templates.rs` 的 `import_one`（约 475-491 行）：`verify_under(root, dir_path)` 后，将路径形式的 `.import-UUID.tmp` 交给 `copy_with_hash`，再路径形式 `rename`；期间没有固定父目录句柄，亦没有在提交前以安全句柄证明目标仍在根内。
  - `src-tauri/src/templates/fsops.rs:92-100`：`O_NOFOLLOW` 只作用于临时文件叶子；内核仍会解析并跟随父目录链接。
  - `handoffs/implementer-mvalfgu5-pn4sn.result.yaml` 明确披露此残余，而人工决策 B 只解决 SPEC-23 展示边界及已接受的非阻断限制，并未接受此安全例外。
- Reproduction / evidence（源码推导的可行交错，未对真实目录或桌面执行攻击）：
  1. 应用对真实 `templates/行政` 完成最后一次 `verify_under`；
  2. 外部进程将该目录改名，并在原名建立指向根外可写目录的链接；
  3. `copy_with_hash` 打开 `templates/行政/.import-UUID.tmp`：父目录链接被跟随，根外临时文件已经创建并写入；
  4. 保持该链接直到后续 `rename`，最终副本仍在根外；叶子不是链接，故 `O_NOFOLLOW` 不拒绝。
  应用的内部 guard 不约束外部文件管理器或进程。即使事后再校验，也无法撤销“已在根外写入”这一事实。此问题需要竞争，不同于上轮无需竞争的原始 S1；风险没有因补上静态校验而消失。
- Required decision: 人工明确目录并发替换是否属于必须防御的威胁模型。若必须保证现有边界，应派发以固定目录句柄/相对句柄操作等方式防止目录替换逃逸的实现及回归，不要求 reviewer 越权修复；若接受残余，需明确记录安全例外及适用操作范围，不能以当前决策 B 代替。

### 实现者第二项披露：分组删除移动符号链接条目

- `service/templates.rs:320-344` 用 `symlink_metadata` 分类，链接按非目录走 `move_file`；同一文件系统 `rename` 移动的是链接本身，而不是目标。**不应把“移动链接条目”直接等同于“跟随链接写入根外”**，本轮不单独据此新增阻断。
- 此分支对杂项条目的保留，与扫描忽略链接、后续访问拒绝链接是不同语义；仍需在安全决策后补充行为测试。`move_file` 的 EXDEV 回退含 `File::open(src)`，不能泛称所有跨文件系统情况都绝不跟随链接；当前未验证挂载/跨设备拓扑，不能把实现者“按 rename 不跟随”的说明扩大成全路径保证。

## Tests Reviewed

- 本轮独立执行：**无**。识别安全风险后依任务停止，未运行 cargo、build 或 E2E；没有新的测试通过声明。
- 实现者结果中的测试及上轮报告的 cargo/build/E2E 均仅为历史证据，不替代本轮复验，尤其不能证明 TOCTOU 安全。
- E2E 后续只能运行 `.specflow/bin/e2e-isolated.sh`；本轮未直接运行 `npm run test:e2e`，未操作真实 KDE 快捷键、剪贴板、自动粘贴或真实应用输入。

## Design Conformance

按 `docs/design/visual-qa-checklist.md` 逐项登记。以下未勾选项均为**安全停止后未完成**，不是已查出的视觉缺陷，也不以构建或旧 E2E 替代专项。

### 一致性
- [ ] 全表面 token / 硬编码颜色圆角 grep：未执行。
- [ ] 全应用强调色、圆角、字体一致性：未复验。

### 主题与对比度
- [ ] 明暗关键面截图：未执行隔离截图；实机部分 `pending_human`。
- [ ] 正文/大字/muted/选中态/主按钮 WCAG 对比度数值：未测。
- [ ] 主题即时切换、无白闪及重启首帧：未复验。

### 字体与圆角
- [ ] 字体真实加载、Figtree 零引用：未复验。
- [ ] 26/18/14/999 圆角规则：未复验。
- [ ] 中文无斜体、衬线标题 ≥18px：未复验。

### 动效与无障碍
- [ ] 动效动机与各窗口时长：未复验。
- [ ] reduced-motion 全部瞬时：未复验。
- [ ] focus-visible 与全键盘操作：未复验；真实焦点操作 `pending_human`。

### 状态完整性
- [ ] loading/empty/error 及错误下一步指引：未复验。
- [ ] 缺失、自动导入标记：未复验；导出提示属于 SPEC-23。
- [ ] 信息密度：未复验。

### 隐私与安全（视觉侧）
- [ ] 敏感正文遮挡及编辑器挂载：未复验。
- [ ] 文件模板无敏感开关、不预览正文：未复验（上轮源码证据不作本轮完成）。
- [ ] 日志/提示无查询词、正文、文件名或路径泄露：未完成全链路审查；目录边界风险见 S1-R。

### 回归
- [ ] 全键盘语义：未复验；文件快搜展示/行为按决策 B 属 SPEC-23，真实 IME `pending_human`。
- [ ] 设置后端持久化：未复验。

### Searchis 专项
- [ ] backdrop-filter、玻璃 token、氛围光零引用：未执行专项 grep。
- [ ] index.css 与设计 token 逐项对应：未复验。
- [ ] 未引用组件、已删除组件零引用：未枚举。
- [ ] 纸张颗粒 pointer-events:none：未复验；真实拖入 `pending_human`。
- [ ] 快搜 Q-A 文件模板展示：依人工决策 B 归 SPEC-23，不作为 SPEC-22 阻断。
- [ ] 文件扩展名/文本语言徽标规则：未复验。
- [ ] 每界面最多一个主按钮：未复验。
- [ ] KWin 圆角及 KDE Global Menu：`pending_human`。

## Next Step

保持 `needs_decision`，由 runtime/人工处理目录级 TOCTOU 安全决策。不自行标记 QA passed、批准或归档。决策或修复后重新派发完整复审，补齐本轮未完成的定向 Rust、build、隔离 E2E、Design Conformance、隐私/无障碍/状态完整性等专项及实机待验项。
