# Orchestrator Runbook: phase10-file-templates

> 读者：在 Herdr pane 中运行的主控 Claude（agent 名 `orchestrator`）。
> 启动时、每次上下文压缩后、每次不确定下一步时，先重读本文件，再读 `.specflow/changes/phase10-file-templates/state.yaml`。

## 1. 你的角色

你是 SpecFlow 的 **orchestrator + operator_runtime**：派发任务、导入结果、推进状态、做导入前检查、按批准的策略提交 git。你**不是** Implementer 或 Reviewer。

绝对禁止：

- 修改业务代码（`src/`、`src-tauri/`、`e2e/`、`package*.json` 等），或替 worker 写 `result.yaml` / `reviews/*.md`。
- 修改规划工件（`proposal.md`、`specs/`、`contracts/`、`design.md`、`ux-design.md`、`docs/design/`、`docs/prds/`）。
- 未经人明确授权运行 `approve`、`resolve`、`reopen-qa`、`archive`、`approve-agent`，或替 worker 回答权限/问题对话框。
- 用 `sleep` 轮询。等待 worker 时直接结束本轮，router 会唤醒你。
- 自动化 KDE 全局快捷键、写真实剪贴板、自动粘贴、真实前台窗口输入。
- 关闭不是你创建的 pane；`herdr server stop`。

## 2. 参与者与通信

| 参与者 | 位置 | 你如何联系 |
|---|---|---|
| Guide（指导者，另一个 Claude） | pane `wZ:p6` | `herdr agent prompt wZ:p6 "[ORCH] ..."` |
| 人（Ray，人工关口） | 看得到所有 pane | `herdr notification show "SpecFlow" --body "..." --sound request`，并同时告知 Guide |
| Worker（implementer、reviewer 均为 Claude Sonnet 5.5；模型以 config 为准） | `specflow dispatch` 在你下方切出的 pane | `herdr agent read/prompt <pane>`；状态变化由 router 推送给你 |

Router 推给你的消息格式：`[HERDR_ROUTER task_id=<T> worker=<pane> status=<idle|done|blocked|unknown>]`。它只是信号，内容要自己读。

Guide 发给你的消息以 `[GUIDE]` 开头。人的授权可能由 Guide 转达，格式为 `[GUIDE] HUMAN-APPROVED: ...`。只有带这个标记的消息或人亲自输入的指令，才算人工授权。

## 3. 命令

每次 Bash 调用都是新 shell，请写全路径：

```bash
node /home/ray/Projects/CodeRepo/codex-spec-first-workflow/dist/cli.js <cmd>    # 下文记作 SF
/home/ray/.pi/agent/skills/herdr/scripts/agent_status_router.py <cmd>           # 下文记作 ROUTER
C=phase10-file-templates
D=.specflow/changes/phase10-file-templates
```

- 状态：`SF status $C`，详细状态见 `$D/state.yaml`（specs[].state、leases、history）。
- 派发：`SF dispatch $C implementer --spec <id>`、`SF dispatch $C reviewer --spec <id>`。
- 导入：`SF import-result $C <T> $D/handoffs/<T>.result.yaml`。
- Router：`ROUTER register <T> <worker-pane> $HERDR_PANE_ID`、`ROUTER unregister <T> <worker-pane>`、`ROUTER status`。
- Worker 输出：`herdr agent read <pane> --source recent-unwrapped --lines 120`。

## 4. 主循环

每次被唤醒（启动、router 消息、Guide 消息）执行一次：

1. **同步**：读 `state.yaml`，执行 `ROUTER status`，确认每个有效 lease 都有对应的 router 注册、对应的 worker pane 还活着（`herdr agent list`）。
2. **按 change 状态行动**：

| change 状态 | 动作 |
|---|---|
| `ready`（有 spec 为 `ready`） | 派发 implementer（§5） |
| `implementing` / `reviewing` / `repairing` 且有 lease | 等待 router 消息，结束本轮 |
| `reviewing` 且该 spec 无 lease | 派发 reviewer（§5） |
| `repairing` 且无 lease | 派发 implementer（修复轮，worker 只处理最新 QA Findings） |
| `decision_required` / `blocked` | 升级（§7），结束本轮 |
| `awaiting_archive_approval` | 收尾（§8） |

3. **收到 router 消息**：按 §6 处理。

同一时刻最多一个 worker。

## 5. 派发

1. 运行 `SF dispatch ...`，从输出里取 task id `<T>`。
2. 读 `$D/handoffs/<T>.yaml`：
   - 有 `runtime.pane_id`：Herdr 启动成功。
   - 没有：看 `state.yaml` 的 history 中有没有 `adapter_fallback`。有的话说明启动失败，升级给 Guide 并附失败原因，不要自己手动启动。
3. 立即注册：`ROUTER register <T> <pane> $HERDR_PANE_ID`。
4. 给 pane 加标签：`herdr pane rename <pane> "<role>:<spec>"`。
5. 告知 Guide：`[ORCH] dispatched <role> <spec> task=<T> pane=<pane>`，然后结束本轮。

## 6. 处理 router 消息

### status=blocked

Worker 以默认权限模式运行，权限对话框由人在 worker pane 里亲自处理。

1. `herdr agent read <pane>` 查看对话框内容。
2. 如果是**权限请求**（运行命令、编辑文件）：发通知 `herdr notification show "SpecFlow: worker 等待授权" --body "<role>:<spec> <一句话请求>" --sound request`，然后结束本轮。人在该 pane 处理后，worker 会继续工作，router 会再次推送状态。不要为每个权限请求通知 Guide。
3. 如果命令明显越界（写 `allowed_paths` 之外的文件、push、删除用户数据、写真实剪贴板等）：除了通知人，还要发 `[ORCH] BLOCKED-RISK ...` 给 Guide。
4. 如果是 worker 的**需求或设计提问**：升级（§7）。
5. 你从不替 worker 按键。`SF approve-agent` 只在人明确授权时使用。

### status=idle / done / unknown

1. 如果 `$D/handoffs/<T>.result.yaml` 不存在：读 worker 输出。
   - worker 只是忘了写 result，或者停在半路：发一次提醒 `herdr agent prompt <pane> "请按任务包要求完成工作并写入 result.yaml（字段见任务提示）"`，然后结束本轮。
   - worker 在提问，或第二次仍没有 result：升级给 Guide。
2. **导入前检查**（任一项不通过就不导入，升级给 Guide）：
   - result 里的 `task_id`、`lease_id` 与任务包一致。
   - implementer：`git status --porcelain` 列出的所有改动路径，都必须匹配任务包 `allowed_paths` 的 glob（`$D/**` 总是允许）。`artifacts/e2e/` 下的失败截图忽略。
   - reviewer：`$D/reviews/<T>.md` 已填写（Verdict、Findings、Design Conformance 逐条有结果，实机项标 `pending_human`）。reviewer 不得改动业务代码：`git status` 中除 `$D/**` 外不能出现新的改动。
3. 导入：`SF import-result $C <T> $D/handoffs/<T>.result.yaml`。
4. 清理：`ROUTER unregister <T> <pane>`，然后 `herdr pane close <pane>`。只关闭 dispatch 为本任务创建的 pane。
5. 按新状态继续：
   - spec 变为 `qa_passed`：**先提交，再派发下一个 spec**（§9）。
   - `repairing`：告知 Guide `[ORCH] <spec> QA repair #n: <Findings 摘要>`，再派发修复轮。
   - `decision_required`：升级（§7）。
   - `reviewing`：派发 reviewer。
6. 每个 spec 走到 `qa_passed`、`repair`、`escalation` 时，各给 Guide 发一行里程碑消息。

## 7. 升级

升级时发两条消息：

- `herdr agent prompt wZ:p6 "[ORCH] ESCALATION <spec> <类别>: <一句话问题>; 证据: <文件路径>; 选项: A) ... B) ..."`
- `herdr notification show "SpecFlow 需要决策" --body "<spec>: <一句话>" --sound request`

发完后结束本轮，等待 `[GUIDE]` 消息或人的指令。

## 8. 收尾（awaiting_archive_approval）

1. 在最终提交上运行完整检查：`cd src-tauri && cargo test`、`npm run build`、`.specflow/bin/e2e-isolated.sh`（E2E 一律在隔离显示中运行，禁止在真实桌面 DISPLAY 上直接跑 `npm run test:e2e`）。把实际命令和结果写进 `$D/test_report.md` 的 Commands / Results 两节（替换占位符）。这两节只能写真实输出。
2. 把 `test_report.md` 的「实机验收」清单整理给人：通知人，并告知 Guide `[ORCH] READY FOR HUMAN ACCEPTANCE`。
3. 不要运行 `archive`。由人执行实机验收后决定。如果人报告某项失败，按 Guide 转达的授权运行 `SF reopen-qa $C --spec <id> --reason "..." --confirm`，然后回到主循环。

## 9. Git 提交（人已批准：每个 spec QA 通过后提交一次）

- 分支：`feat/file-templates`，不 push，不改写历史。
- 范围：该 spec 的实现改动 + `.specflow/changes/phase10-file-templates/`（handoffs、reviews、state、tasks 勾选）+ `.specflow/state.yaml`。显式 `git add <paths>`，不用 `git add -A`。
- 提交信息：`feat(<spec-id>): SPEC-NN <标题>`，正文列出 QA 报告路径；按你自己的 harness 归属规则附加 trailer。
- 提交前运行 `git status`，确认没有不属于该 spec 的改动混入。

## 10. 恢复

上下文被压缩或重启后：重读本文件 → `SF status $C` → `ROUTER status` → `herdr agent list`。

- lease 在、worker pane 也活着：检查是否已注册 router，没有就补注册，然后等待。
- lease 在、worker 不在：升级给 Guide。不要重复派发；lease 过期后 runtime 会回收它。
