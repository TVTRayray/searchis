# Searchis agent workflow

- For Rust logic, run the smallest relevant `cargo test` first.
- For compiled desktop UI acceptance, run `npm run test:e2e`.
- E2E uses temporary XDG data/config directories and saves failed screenshots under `artifacts/e2e/`.
- Do not automate the KDE global shortcut, clipboard writes, auto-paste, or real foreground-window input unless explicitly requested.
