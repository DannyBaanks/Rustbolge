# Rustbolge

Classic Malbolge engine in Rust 1.97 — semantic port, bit-exact, of
`Malbolge-Engine/src/vm.c`, plus the piece the genealogy matrix marks as
NOT_DEMONSTRATED in the fast VMs: **snapshot / resume with JSON
serialization**.

## Why it exists

`GENEALOGIA_MALBOLGE/INTERPRETERS.md` snapshot matrix: Autobolge-Zig is fast
but has NO snapshot; Walbolge/Malbolge-Translator have snapshot/resume but are
Python. Rustbolge fills the "fast + snapshot" cell and implements the
overlay/chain memory of vm.c exactly, so it can serve as a second oracle in
the differential harness.

## Gate (verified 2026-09-11)

| Check | Expected | Got |
| --- | --- | --- |
| `hello_classic.mal` one-shot | `Hello, world.`, 48 steps, HALTED | exact match |
| resume from step-17 snapshot | same finals (a=19758, c=85, d=63) | exact match |
| `cargo test` | 5/5 | PASS |

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
