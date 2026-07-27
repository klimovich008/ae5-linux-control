#!/usr/bin/env python3
"""PreToolUse guard: block shell commands that would set the desktop sink
to a runaway level.

This began as a 20% ceiling, added while testing headphone gain above
32 ohms. That testing is finished and 20% sits below normal listening, so
the ceiling is now a runaway guard rather than a listening limit: it exists
to catch a fat-fingered 100%, not to police volume. Ordinary levels pass.

Note the scope. This only sees shell commands, so it constrains the agent,
not the application — the GUI writes through ALSA and PipeWire directly and
never crosses this hook.

Reads the Claude Code hook JSON on stdin and emits a permission decision on
stdout."""

import json
import re
import sys

CEILING_PERCENT = 60.0

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
                    "(final level unknowable; use an absolute value)"
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
            "BLOCKED by AE-5 runaway-volume guard: "
            + "; ".join(found)
            + f". This catches an accidental full-scale write, not ordinary "
            f"listening levels — anything up to {CEILING_PERCENT:.0f}% passes."
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
