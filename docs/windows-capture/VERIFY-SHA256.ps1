Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$expected = [ordered]@{
    "fixtures-48000/parity-tones.wav" = "7d3c4454021d1264296b8045cc5cf191ae8b7545ae1801ebb414c1c4daf6137b"
    "fixtures-48000/parity-sweep.wav" = "da92e086f0eb48e7a6691c05a9de13cce5f6a3b4d39077c53592577072a5e584"
    "fixtures-48000/parity-level-steps.wav" = "0adbdd2b60a0aca74b8793511d846e5f2d8178d5ec717f0b756cb4eea38b7414"
    "fixtures-48000/parity-silence.wav" = "06045062d0c6a0994ea36efd87f62499abd07fd473de2737c12d04b14dbaf808"
    "fixtures-48000/parity-channel-id-6ch.wav" = "fd1b267e9227af4f1262d58044c6a3a8548847157d73c69a9c0bc670c7f6c32a"
    "tools/audacity-win-3.7.7-64bit.zip" = "1d345a48a698c57363475b7b3a0b113f796c2a49c6254d5f41dae53b9f4017d8"
}

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
foreach ($relativePath in $expected.Keys) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing handoff file: $relativePath"
    }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected[$relativePath]) {
        throw "SHA-256 mismatch: $relativePath"
    }
    Write-Host "OK  $relativePath"
}

Write-Host "All 6 handoff files verified"
