#!/usr/bin/env python3
"""Measure harmonic distortion in a PCM WAV capture of a steady tone."""

import argparse
import math
import sys
import wave
from pathlib import Path

import numpy as np


def load_first_channel(path: Path) -> tuple[np.ndarray, int]:
    with wave.open(str(path), "rb") as recording:
        sample_rate = recording.getframerate()
        sample_width = recording.getsampwidth()
        channels = recording.getnchannels()
        frames = recording.getnframes()
        payload = recording.readframes(frames)

    if sample_width not in (2, 4):
        raise ValueError(f"expected 16-bit or 32-bit PCM, got {sample_width * 8}-bit")
    if channels < 1:
        raise ValueError("capture has no channels")

    dtype = "<i2" if sample_width == 2 else "<i4"
    samples = np.frombuffer(payload, dtype=dtype)
    if samples.size % channels:
        raise ValueError("capture payload is not channel-aligned")
    first = samples.reshape(-1, channels)[:, 0].astype(np.float64)
    return first / float(2 ** (8 * sample_width - 1)), sample_rate


def measure(path: Path, tone_hz: float) -> dict[str, float | int]:
    samples, sample_rate = load_first_channel(path)
    window_frames = 1 << 15
    if samples.size < window_frames:
        raise ValueError(
            f"capture has {samples.size} frames; at least {window_frames} are required"
        )
    if not 20.0 <= tone_hz < sample_rate / 2:
        raise ValueError(f"tone frequency {tone_hz:g} Hz is outside the measurable range")

    start = samples.size // 2 - window_frames // 2
    segment = samples[start : start + window_frames]
    peak_percent = float(np.max(np.abs(segment)) * 100.0)
    segment = (segment - np.mean(segment)) * np.hanning(window_frames)
    spectrum = np.abs(np.fft.rfft(segment))
    frequencies = np.fft.rfftfreq(window_frames, 1.0 / sample_rate)

    def peak(target_hz: float, width_hz: float = 25.0) -> float:
        selected = (frequencies > target_hz - width_hz) & (
            frequencies < target_hz + width_hz
        )
        return float(np.max(spectrum[selected])) if np.any(selected) else 0.0

    fundamental = peak(tone_hz)
    if not math.isfinite(fundamental) or fundamental <= 0.0:
        raise ValueError("fundamental was not found")

    harmonics = [
        peak(tone_hz * multiple)
        for multiple in range(2, 7)
        if tone_hz * multiple < sample_rate / 2
    ]
    ratio = math.sqrt(sum(value * value for value in harmonics)) / fundamental
    if not math.isfinite(ratio):
        raise ValueError("THD result is not finite")

    result: dict[str, float | int] = {
        "sample_rate": sample_rate,
        "frames": samples.size,
        "signal_peak_percent": peak_percent,
        "thd_percent": ratio * 100.0,
    }
    for index, value in enumerate(harmonics, start=2):
        result[f"harmonic_{index}_dbc"] = (
            20.0 * math.log10(value / fundamental) if value > 0.0 else -math.inf
        )
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("capture", type=Path)
    parser.add_argument("--tone-hz", type=float, default=1000.0)
    arguments = parser.parse_args()

    try:
        result = measure(arguments.capture, arguments.tone_hz)
    except (OSError, ValueError, wave.Error) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    for name, value in result.items():
        if isinstance(value, float):
            print(f"{name}={value:.9f}")
        else:
            print(f"{name}={value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
