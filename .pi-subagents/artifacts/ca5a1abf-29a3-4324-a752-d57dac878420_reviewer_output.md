## Review

### Required fixes

- **High — Revision conflicts have no reload operation.**  
  `snippet_get` exists at `front-baseline/src/api/snippets.ts:58`, but the UI only displays the error at `front-baseline/src/App.tsx:101-103,191-195`; clicking the current sidebar item merely reloads stale React state at `App.tsx:145-153`. This violates the explicit reload-action contract in `docs/prds/prd.md:495-496` and `.agent/specs/01-secure-persistent-snippet.md:67`. Add a reload action that fetches the current record and deliberately handles unsaved input.

- **Medium — Idempotency can silently accept a different payload.**  
  `create_requests` stores only `request_id → snippet_id` (`front-baseline/src-tauri/src/storage.rs:52-64,323-326`). The UI retains the same request ID after failures while allowing edits (`front-baseline/src/App.tsx:28,82-103`). If the first write committed but its response was lost, a changed retry returns the old snippet and `selectSnippet(saved)` discards the changed draft. Bind the idempotency record to a request fingerprint and reject mismatched reuse, or otherwise prevent silent payload substitution.

- **Medium — Key is stored in the data directory instead of the required configuration directory.**  
  `front-baseline/src-tauri/src/lib.rs:18-22` passes only `app_data_dir`, and `storage.rs:30` places `database.key` there. PRD NFR 11.3 requires the independent key in the application configuration directory (`docs/prds/prd.md:642-647`).

- **Medium — Key creation is not crash-durable at the directory-entry level.**  
  `front-baseline/src-tauri/src/storage.rs:398-412` uses `create_new`, writes, and `sync_all`s the file, but never fsyncs its parent directory before creating/committing the encrypted database. A power loss can therefore leave a persistent encrypted database without its only key. Fsync the containing directory and add focused durability/failure-path coverage.

- **Medium — Required diagnostic logging is incomplete.**  
  Only startup failure code and environment values are written to stderr (`front-baseline/src-tauri/src/lib.rs:27-41`). CRUD errors are directly propagated without logging (`front-baseline/src-tauri/src/commands.rs:27-50`), and logs lack timestamp and application version. This does not satisfy `docs/prds/prd.md:661-669`.

- **Medium — Navigation remains enabled during an in-flight save and can lose a new draft.**  
  Saving captures the old selection and later unconditionally calls `selectSnippet(saved)` (`front-baseline/src/App.tsx:82-105`), while “New” and sidebar buttons remain enabled (`App.tsx:134-153`). A user can start another draft while saving and have it replaced when the earlier request completes. Disable navigation or ignore stale completions.

- **Low — Symlink rejection has a check/open race.**  
  `front-baseline/src-tauri/src/storage.rs:369-385` checks with `symlink_metadata` and then separately opens the path without `O_NOFOLLOW`; the path can change between those operations. Validate metadata from the opened descriptor and use `O_NOFOLLOW`. The current tests cover missing/wide permissions but not symlink or ownership rejection (`storage.rs:476-500`).

- **Low — Claimed boundary coverage is incomplete.**  
  The UTF-8 test only checks an oversized value (`front-baseline/src-tauri/src/validation.rs:161-177`), not the required exactly-102400-byte accepted boundary from `docs/prds/prd.md:687-689`. Transaction rollback is exercised only for create (`storage.rs:559-580`), not update. There are no frontend tests for conflict reload, input retention, or in-flight navigation.

### Correct

- **No blocker found in the SQLCipher selection itself.** `front-baseline/src-tauri/Cargo.toml:23` selects bundled SQLCipher with vendored OpenSSL; `storage.rs:275-295` applies the binary key, checks `cipher_version`, and verifies the database before migration.
- CSPRNG generation, `create_new`, `0600`, owner, regular-file and length checks are present at `storage.rs:368-412`; key/content are not logged.
- Create/update/schema operations use transactions, user values are parameterized, the normalized-key unique index covers all rows, and update uses `WHERE id AND revision` (`storage.rs:48-160,297-353`).
- Startup database failure produces a read-only error page; save errors retain form state; sensitive content is masked from the rendered DOM by default (`front-baseline/src/App.tsx:112-124,171-176,191-202`).
- No `dangerouslySetInnerHTML` was found.

### QA-manual items

After required fixes:

1. On target Arch/KDE Plasma 6/X11, create newline/Emoji/combining-character content, fully exit, restart, verify metadata/content, then edit and restart again.
2. Execute AC-02 for active and soft-deleted key conflicts and confirm record count remains unchanged.
3. Exercise two-window/process revision conflict and verify the new reload operation works without silently merging.
4. Verify key location, owner, exact mode, startup error guidance, and ordinary SQLite rejection on the target package snapshot.
5. Desktop packaging and interactive Tauri launch were not independently rerun; only Rust checks and frontend production build were run.

### Scope note

`git status` also reports untracked `docs/prds/**`, other specs, workflow documents, and `.pi-subagents/**`, although SPEC-01 permits only its own spec/master-plan plus implementation paths (`.agent/specs/01-secure-persistent-snippet.md:10-11`). Git cannot establish their provenance because they are untracked; keep them out of the SPEC-01 commit unless separately authorized. No out-of-scope business feature was found in the inspected frontend/Rust implementation.