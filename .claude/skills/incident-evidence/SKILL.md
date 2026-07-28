---
name: incident-evidence
description: Evidence-first response when AE-5 audio misbehaves (loud buzz, effects not applying, unexpected silence, stale DSP state). Use BEFORE attempting any recovery so the fault can be diagnosed.
---

# AE-5 incident evidence collection

A reproducible fault is worth more than a fast recovery. Collect evidence
FIRST, recover SECOND. (Precedent: the 2026-07-26 S32 loud-buzz trigger
was lost because no instrumentation existed at the moment it fired.)

## 1. If sound is actively loud/dangerous

Hardware mute immediately — this is the only step allowed before capture:

```sh
ae5ctl set-playback-switch Master off
```

## 2. Capture state before touching anything else

Save all of this into `/tmp/ae5-incident-$(date +%Y%m%d-%H%M%S)/`:

```sh
dmesg | tail -200                                   # kernel messages
journalctl --user -u pipewire -u wireplumber -n 200 --no-pager
for f in /proc/asound/card0/pcm*p/sub*/{status,hw_params,sw_params}; do
    echo "== $f"; cat "$f" 2>/dev/null; done          # PCM state
amixer -c 0 contents                                 # full raw mixer readback
pw-dump > pw-dump.json                               # PipeWire graph
wpctl status                                         # desktop routing/volume
ae5ctl status; ae5ctl route-status                   # project view
cat /proc/sys/kernel/tainted; uname -r
bash scripts/collect-routing-state.sh 2>/dev/null    # project collector if usable
```

Also record: what was playing, which client, exact user action (e.g.
track switch), and wall-clock time.

## 3. Compare mixer readback against the selected profile

The GTK app / `ae5ctl` can verify the applied profile (21-control
compare). Record which controls diverge — do not fix them yet.

## 4. Only then recover

With the physical output hard-muted: toggle global OutFX off, reapply the
intended profile, verify all controls match, then restore the safe state
(sink muted at 5%, per `/audio-safety-preflight`).

## 5. Report

Summarize the evidence location, the divergent controls, and whether the
fault matches a known signature (S32 track-switch buzz, hidden
Master-mute split, stale Smart Volume after resume) before proposing a
fix. Update `docs/` only with conclusions the evidence supports.
