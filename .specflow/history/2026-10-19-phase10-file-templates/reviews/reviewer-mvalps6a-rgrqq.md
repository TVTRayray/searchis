# QA Report: reviewer-mvalps6a-rgrqq

## Verdict

- `escalation` / `security_risk`；工作单结果 `needs_decision`。
- 本轮没有修改实现、state.yaml、tasks.md 或 test_report.md；不标记 QA passed，不批准或归档。
- contracts/22「安全与隐私」最后一条接受的**目录级 TOCTOU**不再列为阻断。下述问题是无竞态的静态符号链接跟随，不属于该例外。

## Inputs / Scope

已复审 proposal.md、tasks.md、specs/file-templates.md、design.md、ux-design.md、contracts/22-file-template-management.md，并参考既有复审记录。写入范围仅本 change 的 reviews/ 与 handoffs/。没有创建 subagent；没有操作真实桌面、全局快捷键、真实剪贴板、自动粘贴或外部应用输入。

## Spec Coverage

| 场景 | 证据 | 结果 |
| --- | --- | --- |
| S1 静态中间链接逃逸 | paths::verify_under 自 root 逐段 symlink_metadata + canonical containment；safe_join/ensure_dir_in；service/templates.rs 静态目录链接测试 | 常规受校验路径的修复成立，相关测试通过；不能据此宣称所有入口均无静态逃逸 |
| 路径遍历/链接拒绝、目录创建校验 | templates::paths / fsops 单测、模板服务回归 | 通过现有测试 |
| 文件模板导入、编辑、打开、回收、恢复、扫描及分组同步 | Rust 模板测试与隔离 E2E 的 SPEC-22 场景 | 通过现有测试 |
| 分组删除的链接条目同设备移动 | fs_group_delete 对非目录条目加入移动计划，调用 move_file；rename 不跟随叶子链接 | 保留链接本身，不读取其目标内容 |
| 分组删除的链接条目跨设备移动 | 本轮独立 EXDEV 复现，见下文 | **不通过**：回退跟随链接，将目标内容复制为普通文件 |
| 目录级 TOCTOU 残余 | contracts/22 人工例外 | 已接受，不阻断、不重复升级 |
| 实机检查 | 本轮未运行真实桌面相关操作 | pending_human |

## Findings

### S2 — EXDEV 回退跟随静态链接并将目录外内容实体化

- Severity: high（有跨设备前提）。
- Category: `security_risk`，不是仅仅 `implementation_defect`。
- Expected: 分组删除的跨设备回退不得读取符号链接指向的目录外内容。若允许移动链接，应保留链接本身；或在发生该类条目时明确拒绝并保持操作/回滚一致。选择哪种策略须由决策方确认。
- Actual:
  1. `src-tauri/src/service/templates.rs::fs_group_delete` 将目录内非目录条目（含链接）交给通用移动函数。
  2. `src-tauri/src/templates/fsops.rs:149–179`：`rename` 遇 EXDEV 转 `copy_verify_remove`。
  3. `copy_with_hash` 使用 `File::open(src)`（约第 95 行），跟随叶子符号链接；`O_NOFOLLOW` 只用于新建目标临时文件，不能保护源。
  4. 临时副本校验通过后 rename 为目标，再 remove_file 删除原链接。结果是链接目标内容被读入模板目录中的普通文件；原目标本身未被改写或删除。
- Trigger: 分组目录位于另一个挂载/设备，内含指向目录外普通文件的静态链接，删除分组需将该条目上移到另一设备。无并发替换、不依赖 TOCTOU。
- Security judgment: 可以把原本应被扫描跳过/路径拒绝的外部文件内容实体化到模板目录，后续可被扫描纳入。这是源读取边界失守，而不仅是链接类型未保留。风险是非预期读取/复制；**没有证据表明该分支改写或删除外部目标**。不将纯链接保留、一般分组删除或所有 EXDEV 操作一概认定为漏洞。
- Reproduction:
  - `reviews/evidence-reviewer-mvalps6a-rgrqq/exdev-link-repro.rs` 从现有 fsops 原样抽取 ensure_dir、sha256_file、copy_with_hash、ModeExt、move_file、copy_verify_remove；仅错误类型简化为 String。
  - 用 `/tmp` 与 `/dev/shm` 的独立临时目录、合成 `EXTERNAL_SENTINEL` 文件验证实际 rename 返回 EXDEV，再调用上述移动函数；没有读取用户私有文件。结束后清理两个临时目录。
  - `exdev-link-repro.log`：`EXDEV confirmed; destination_is_symlink=false; destination_content=EXTERNAL_SENTINEL; source_link_exists=false; external_target_unchanged=true`。
  - 这是实际 helper 分支复现，不冒称整个分组删除端到端测试；调用链由静态审查确认。
- Test gap: 现有全绿测试没有覆盖「分组删除 + 静态链接 + EXDEV」。修复后应补充源链接处理、目标不被覆盖、失败回滚/数据库一致性，以及外部目标不被读取/改写的回归用例。

## Test Results

本轮实际执行（不是引用 implementer 的历史结果）：

- `cd src-tauri && cargo test template`：通过。
- `cd src-tauri && cargo test`：139 passed，0 failed；main/doc tests 0 failed。
- `npm run build`：通过（tsc + Vite）；存在大 chunk 提示，不是构建失败。
- `.specflow/bin/e2e-isolated.sh`：退出 0；4 passing，1 spec passed，包括 SPEC-22 导入/编辑/打开/回收/重扫。使用 Xvfb + 私有 D-Bus + KWin，未直接在真实 DISPLAY 调用 test:e2e。
- EXDEV 原函数抽取复现：成功复现上列静态链接内容复制，证据文件已保存。首次编译漏传 `--extern hex` 未成功，补齐依赖后编译并运行成功；没有更改产品源码。
- 非阻断环境诊断：Rust `group_name` dead_code warning；E2E 存在 KDE portal/KCrash 诊断，但测试退出 0。不能据此声称真实桌面 portal 正常。

## Design Conformance

设计影响为 design_system。依据 docs/design/visual-qa-checklist.md 逐项记录。`[x]` 仅代表所注明的静态/自动化层次，不代表双主题截图或完整视觉验收。发现安全风险后按工作单停止，不把未验证项伪记通过。

### 一致性
- [x] token 体系：执行 `grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components src/App.tsx` 无匹配；新增模板组件使用语义 class/token。此 grep **不证明圆角与阴影全部合规**。
- [ ] 强调色/圆角/字体全应用一致：已检查新模板组件和主题定义，未完成全窗口视觉验证。

### 主题与对比度
- [ ] 明/暗关键面截图对比：本轮无双主题成功截图证据，未做像素视觉验收。
- [ ] 正文、大字、muted、选中态、主按钮对比度：未完成本轮全矩阵计算/渲染检查，不沿用设计文档估值作 QA 通过。
- [ ] 主题即时切换、无闪烁及重启首帧：未专项复验。

### 字体与圆角
- [x] 字体发货与旧引用静态检查：构建实际产出 Source Sans 3、Playfair Display、JetBrains Mono 字体资源；源码搜索未见 Figtree。运行时真实字体加载仍未专项验收。
- [ ] 全应用圆角 26/18/14/999：未逐界面验证。
- [ ] 中文无斜体、衬线仅 ≥18px：新增模板组件未见主动中文斜体，未完成全应用检查。

### 动效与无障碍
- [ ] 动效时限和动机：未完成逐交互计时检查。
- [ ] reduced-motion 全部瞬时：未专项实测。
- [ ] focus-visible 光环与键盘全程：PlusMenu/Dialog 可见 role/aria 与键盘处理；E2E 有基础键盘回归，不能替代所有焦点路径专项。

### 状态完整性
- [x] 模板组件有扫描/导入 busy、空态、缺失态和错误反馈实现；SPEC-22 E2E 覆盖主路径。所有失败态的视觉呈现未穷举。
- [ ] 缺失/自动导入/导出路径提示的全部可见状态：有相应实现，未逐状态视觉验收。
- [ ] 信息密度 6：需关键面视觉对照，本轮不宣称通过。

### 隐私与安全（视觉侧）
- [ ] 敏感正文遮挡/编辑器不挂载：本轮未专项复验。
- [x] FileTemplateDetail 不提供敏感开关，不内联预览文件内容；显示元数据及动作。
- [ ] 日志/提示不含查询、正文、模板名称/路径（导出路径例外）：未完成全调用路径审计。

### 回归
- [ ] 全部快捷键和 IME：现有 E2E 通过不等于列出的所有组合键与 IME 都复验；真实全局操作 pending_human。
- [x] 文件模板业务通过 API/后端持久化；不是 localStorage 作为数据真相源。完整设置专项未重跑。

### Searchis 专项
- [x] 执行源码搜索未见 backdrop-filter、Figtree、HeaderBar、canvas-glow、surface-glass；新增文件组件未引入玻璃样式。
- [ ] index.css 与 token 文档逐值对齐：未完成全表机械比较。
- [ ] 每个组件导出至少一处 import：未完成全组件清单检查，不能以 HeaderBar 零引用替代。
- [ ] 纸张颗粒 pointer-events:none：未做本轮拖入遮挡专项验收。
- [ ] 快搜结果 Q-A 全部元素与高亮：现有回归通过，未逐元素视觉对照。
- [x] 文件类型以图标及大写扩展名表达（FileTypeIcon/FileTemplateDetail）；文本语言徽标全矩阵未专项复验。
- [ ] 每界面最多一个主按钮：新增菜单/对话框结构已阅，未遍历所有界面组合。
- [ ] KWin 26px 圆角及 KDE Global Menu 五组菜单：pending_human；隔离 E2E 不替代本机验收。

## Unresolved Issues / Next Step

1. **需人工决策**：上述静态链接 EXDEV 源读取风险不在目录级 TOCTOU 安全例外之内；确认链接策略后重新派发实施/复审。Reviewer 不在权限外修代码。
2. Design Conformance 已给出逐项证据边界，但未完整验收；安全升级后停止，没有补拍成功截图或宣称视觉通过。后续复审需补齐未验的自动化/设计项，实机项保持 pending_human。
3. 不自行改变 runtime 状态或标记 QA passed。
