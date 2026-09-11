use rustbolge::{load_program, run, Machine, HaltReason, Snapshot, MEM_SIZE};

/// Canonical hello-world fixture, byte-identical to
/// Antivirusbolge/corpus/benign/hello_classic.mal (the same file and SHA256
/// the acceptance rules demand).
const HELLO: &[u8] = include_bytes!("fixtures/hello_classic.mal");

#[test]
fn hello_world_gate() {
    let cells = load_program(HELLO).expect("load");
    let o = run(&cells, &[], 100_000_000);
    let stdout = String::from_utf8_lossy(&o.output);
    assert_eq!(stdout, "Hello, world.", "Classic gate: exact Malbolge-Engine signature");
    assert_eq!(o.halt_reason, HaltReason::VInstruction);
}

#[test]
fn loader_rejects_binary() {
    assert!(load_program(b"(= \n\t ]}").is_ok());
    assert_eq!(load_program(&[0x07]).unwrap_err(), 0x07);
    assert!(load_program(&[0xC3, 0xA9]).is_err()); // UTF-8 is not Classic
}

#[test]
fn memory_bounds() {
    assert_eq!(MEM_SIZE, 59049);
}

#[test]
fn snapshot_resume_is_bitexact() {
    let cells = load_program(HELLO).expect("load");

    // One-shot reference run
    let full = {
        let mut m = Machine::new(&cells);
        m.run(&[], 1_000_000);
        m
    };

    // Split run: 17 steps, snapshot through JSON, resume to completion
    let mut m1 = Machine::new(&cells);
    m1.run(&[], 17);
    let json = serde_json::to_string(&m1.snapshot()).unwrap();
    let snap: Snapshot = serde_json::from_str(&json).unwrap();
    let mut m2 = Machine::from_snapshot(&snap);
    m2.run(&[], 1_000_000);

    assert_eq!(full.output, m2.output, "stdout stream must be identical");
    assert_eq!(full.steps, m2.steps, "step accounting must match");
    assert_eq!(full.a, m2.a);
    assert_eq!(full.c, m2.c);
    assert_eq!(full.d, m2.d);
    assert_eq!(full.halted, m2.halted);
}

#[test]
fn snapshot_halt_survives_json() {
    let cells = load_program(HELLO).expect("load");
    let mut m = Machine::new(&cells);
    m.run(&[], 1_000_000);
    assert!(m.halted);
    let json = serde_json::to_string(&m.snapshot()).unwrap();
    let m2 = Machine::from_snapshot(&serde_json::from_str(&json).unwrap());
    assert_eq!(m2.halt_reason, Some(HaltReason::RestoredHalted));
}
