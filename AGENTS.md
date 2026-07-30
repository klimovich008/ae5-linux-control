# AE-5 Linux Control agent guide

Read [README.md](README.md) for the current product surface and
[ROADMAP.md](ROADMAP.md) for ordered milestones. Investigation narratives in
[GOAL.md](GOAL.md) and older session documents are historical evidence, not
current operating instructions.

## Mission and scope

The primary goal is Windows-equivalent or better daily audio on the exact
Creative Sound BlasterX AE-5 at PCI `1102:0012`, subsystem `1102:0051`.
Preserve this hardware boundary unless a separately audited revision is added.

The production architecture is:

- Rust for discovery, ALSA/PipeWire control, profiles, transactions, and state;
- an unprivileged `ae5d` user service with a typed D-Bus interface;
- Qt 6, Qt Quick, QML, and CXX-Qt for the desktop interface;
- ALSA for card controls, PipeWire for routing/software processing, udev for
  discovery, and systemd user services for activation/restoration.

Keep backend behavior independent of QML so the UI can be redesigned without
rewriting hardware logic.

## Audio and hardware rules

- The application volume range is 0–100%. There is no project-level 20%
  product cap.
- Do not choose an audible test level on the user's behalf. Preserve the
  current volume/mute state for silent control tests; let the user set the
  listening level for an audible comparison.
- Before hardware Effects, route, format, kernel, or DSP-recovery testing,
  mute PipeWire, switch ALSA `Master` and `Front` off, inspect the active card
  index, and confirm the intended output and gain. Keep these hard mutes until
  the checked transaction and diagnostics finish.
- `S32LE` remains outside the managed desktop transport until the documented
  track-change fault has a separate physical acceptance result.
- Preserve logs and live mixer readback before recovering from unexpected
  audio.
- Change headphone gain only through the checked `ae5d` transaction while the
  ALSA and PipeWire routes both identify Headphones. High gain requires an
  explicit user confirmation; preserve and restore the exact stream, mute,
  and mixer state around the write.
- Current direct-filter-v2 software EQ has no preamp stage. Boosted curves can
  clip near full scale. Keep v1 parsing only for fail-safe rollback and
  migration; never reactivate a legacy attenuated graph as a new apply.

Hardware OutFX is a supported, opt-in backend only when all four gates pass:

1. the binary was built with `outfx-lab`;
2. `uname -r` is exactly `7.1.4-ae5-outfx-lab`;
3. `/sys/module/snd_hda_codec_ca0132/parameters/ae5_unsafe_outfx_lab` is `Y`;
4. the `ae5d` process receives
   `AE5_OUTFX_LAB=I_ACCEPT_AE5_DSP_CORRUPTION`.

Do not weaken or bypass these gates. Normal kernels and normal daemon sessions
must fail closed.

Apply hardware Effects only through the whole-profile transaction in
`src/hardware_effects.rs`: park exact-sink streams, suspend the output, disable
master/children, write mode and levels, restore child switches, enable the
master last, verify all values, and roll back while output is paused on any
failure. Never stack the PipeWire software-Effects fallback with hardware
OutFX. Software EQ is a separate processing group and may remain active with
hardware OutFX.

## Development workflow

- Preserve unrelated local changes and never use destructive Git commands.
- Use the current working branch unless the user explicitly asks for a new
  branch.
- Prefer small transactions with readback and rollback over direct mixer
  writes.
- A profile save changes storage only; Apply changes live audio.
- Every live write must surface applying, confirmed, externally changed, or
  failed/rolled-back state to the UI and journal.
- Use Wayland for native GUI smoke tests; retain X11 compatibility.
- Keep `feature-parity.tsv`, README claims, and UI capability text consistent.

## Validation before publishing

Run the focused test for the changed path, then:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
bash scripts/check-ae5-acp-profile.sh
bash scripts/check-feature-parity.sh
bash scripts/audio-parity.sh --self-test
bash scripts/check-user-install.sh
```

For QML changes also run:

```sh
bash scripts/check-qml-accessibility.sh target/release/ae5-control-qml
```

A VM compile/boot is useful for kernel and packaging checks but never replaces
the exact physical-card gate for an audio claim.
