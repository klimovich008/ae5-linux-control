---
name: validate-gate
description: Run the full AE-5 local validation gate (fmt, clippy, tests, ACP/feature-ledger/audio/install validators) required before publishing any change. Use before every commit or push.
---

# AE-5 local validation gate

Run all of these from the repo root; every one must pass before a change
is committed or pushed. Run them in this order (cheap first):

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
bash scripts/check-ae5-acp-profile.sh
bash scripts/check-feature-parity.sh
bash scripts/audio-parity.sh --self-test
bash scripts/check-user-install.sh
```

Additional gates when the change touches:

- **packaging/**: `bash scripts/build-rpm.sh`. Never run
  `scripts/check-rpm-lifecycle.sh` on the host — root-only, disposable
  container only (it refuses the host by design; hosted CI covers it).
- **kernel/**: follow `kernel/README.md` + `docs/KERNEL_MAINTENANCE.md`.
  The patch queue must round-trip apply/reverse cleanly; module-only
  compile uses `W=1 KCFLAGS=-Werror`. A QEMU/VFIO result never substitutes
  for the physical cold-boot/suspend gates.
- **shell scripts**: `bash -n` each changed script plus ShellCheck.
- **feature-parity.tsv**: the 54-row ledger validator must stay green and
  every classification change needs its evidence line updated.

Report pass/fail per step with the failing output verbatim. Never describe
a partially passing gate as passing.
