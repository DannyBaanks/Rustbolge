# Rustbolge v2.0 — Classic Malbolge Engine in Rust 1.97 with Bit-Exact Snapshot/Resume

[![Version](https://img.shields.io/badge/version-2.0-blue.svg)](https://github.com/DannyBaanks/Rustbolge)
[![Rust](https://img.shields.io/badge/rust-1.97%2B-orange.svg)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-5%2F5%20PASS-brightgreen.svg)]()

Classic Malbolge engine in Rust 1.97 — semantic port, bit-exact, of
`Malbolge-Engine/src/vm.c`, plus the piece the genealogy matrix marks as
NOT_DEMONSTRATED in the fast VMs: **snapshot / resume with JSON
serialization**.

Part of the **v2.0 Malbolge Ecosystem Multi-Oracle Consensus Fleet** alongside
`Zigbolge v2.0`, `Swiftbolge v2.0`, `Cobolge v2.0`, and `Fortranbolge v2.0`.

## Why it exists

`GENEALOGIA_MALBOLGE/INTERPRETERS.md` snapshot matrix: Autobolge-Zig is fast
but has NO snapshot; Walbolge/Malbolge-Translator have snapshot/resume but are
Python. Rustbolge fills the "fast + snapshot" cell and implements the
overlay/chain memory of vm.c exactly, so it can serve as a second oracle in
the differential harness.

## v2.0 Breakthroughs & Gates

| Check | Expected | Got |
| --- | --- | --- |
| `hello_classic.mal` one-shot | `Hello, world.`, 48 steps, HALTED | exact match |
| resume from step-17 snapshot | same finals (a=19758, c=85, d=63) | exact match |
| `cargo test` | 5/5 | PASS |
| JSON Serialization | Full memory chain/overlay dump & restore | byte-compatible with Swiftbolge |

Evidence: `evidence/hello_step17_snapshot.json`,
SHA-256 `29372AAD12CAA5AFD340328D6C9AFB51408A3A60A477BDA94A671A0DD79A2FE4`.

Evidence: `evidence/hello_step17_snapshot.json`,
SHA-256 `29372AAD12CAA5AFD340328D6C9AFB51408A3A60A477BDA94A671A0DD79A2FE4`.

## Usage

```
rustbolge <program.mal> [max_steps] [--snapshot-out f] [--resume f] [--json] < input
```

- Compatible with `malbolge.c` CLI: whitespace in source ignored, non-printable
  byte -> error exit 2, `max_steps` default 100M.
- `--snapshot-out f` — dump full VM state (a, c, d, chain, overlay, input
  position, output) as JSON after the run.
- `--resume f` — restore from a snapshot and continue; output accumulates.
- `--json` — JSON run report on stderr (steps, status, halt_reason, finals).

## Halt reasons

`VInstruction` | `InvalidCell` | `Eof` | `StepsExhausted` | `RestoredHalted`

## Limits (KNOWN_LIMIT)

Loader, encryption table and EOF-halt behavior follow Malbolge-Engine vm.c;
`oracle` treats EOF as a=59048 in some builds — expected divergence point for
the differential harness, documented up front. No Unshackled extensions:
strictly 59049 cells.
