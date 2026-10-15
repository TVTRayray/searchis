# Searchis agent workflow

- For Rust logic, run the smallest relevant `cargo test` first.
- For compiled desktop UI acceptance, run `npm run test:e2e`.
- E2E uses temporary XDG data/config directories and saves failed screenshots under `artifacts/e2e/`.
- Do not automate the KDE global shortcut, clipboard writes, auto-paste, or real foreground-window input unless explicitly requested.

## SpecFlow workflow

- Project rules live in `.agent/AGENT.md` (constitution); workflow state lives in `.specflow/` — a change's `state.yaml` is authoritative, not chat or `.agent/master_plan.md` (Phase 1–9 archive only).
- Only do the role and target spec named in your task package `.specflow/changes/<change>/handoffs/<task-id>.yaml`; read its inputs plus the matching `contracts/NN-*.md`.
- Only modify files in the task's `allowed_paths`; "仅限" limits in the contract also apply.
- Never mark QA passed, approve, or archive yourself; write `handoffs/<task-id>.result.yaml` (Reviewer also fills `reviews/<task-id>.md`) and let the runtime advance state.
- Subagents are forbidden unless the task package is a read-only subagent task dispatched by the orchestrator.
- On ambiguous requirements, architecture/security risk, out-of-scope edits, or blockers: write `outcome: needs_decision` (or `blocked`) in the result and stop.
- On-device checks (KDE shortcut, real clipboard, auto-paste, external-app paste, device screenshots) are marked `pending_human` in QA, never automated without explicit approval.
- CLI: `node /home/ray/Projects/CodeRepo/codex-spec-first-workflow/dist/cli.js <command>`.
