#!/usr/bin/env python3
"""PreToolUse guard: block shell commands that would raise the desktop
sink above the project's 20% safety ceiling (HANDOVER.md, non-negotiable
audio safety). Reads the Claude Code hook JSON on stdin and emits a
permission decision on stdout."""

import json
import re
import sys

CEILING_PERCENT = 20.0

VOLUME_CMD = re.compile(
    r"\b(?:wpctl\s+set-volume|pactl\s+set-sink-volume|pamixer\b[^|;&]*--set-volume)\b[^|;&\n]*"
)
PERCENT_TOKEN = re.compile(r"(\d+(?:\.\d+)?)%(?!\S*[-])([+]?)")
FLOAT_TOKEN = re.compile(r"(?<![\w.%])(\d+\.\d+)(?![\w%])")


def violations(command: str):
    found = []
    for m in VOLUME_CMD.finditer(command):
        seg = m.group(0)
        for pm in PERCENT_TOKEN.finditer(seg):
            value, plus = float(pm.group(1)), pm.group(2)
            if plus:
                found.append(
                    f"relative volume raise `{pm.group(0)}` in `{seg.strip()}` "
                    "(final level unknowable; use an absolute value at or below 20%)"
                )
            elif value > CEILING_PERCENT:
                found.append(f"volume {value:g}% > {CEILING_PERCENT:g}% ceiling in `{seg.strip()}`")
        for fm in FLOAT_TOKEN.finditer(seg):
            value = float(fm.group(1))
            if 0.0 < value <= 4.0 and value > CEILING_PERCENT / 100.0:
                found.append(
                    f"linear volume {value:g} (~{value * 100:g}%) > {CEILING_PERCENT:g}% "
                    f"ceiling in `{seg.strip()}`"
                )
    return found


def main() -> None:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return
    command = (data.get("tool_input") or {}).get("command", "")
    if not isinstance(command, str) or not command:
        return
    found = violations(command)
    if found:
        reason = (
            "BLOCKED by AE-5 audio-safety hook: "
            + "; ".join(found)
            + ". HANDOVER.md rule: never raise the PipeWire sink above 20% during "
            "testing (headphone-damage risk). Use <= 20%, e.g. "
            "`wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%`."
        )
        print(
            json.dumps(
                {
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "deny",
                        "permissionDecisionReason": reason,
                    }
                }
            )
        )


if __name__ == "__main__":
    main()
