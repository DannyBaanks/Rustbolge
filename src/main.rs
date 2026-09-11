//! rustbolge CLI
//!
//!   rustbolge <program.mal> [max_steps] < input
//!     --snapshot-out <file>   dump final VM state as JSON
//!     --resume <file>         resume from a snapshot JSON
//!     --json                  emit a JSON run report on stderr

use rustbolge::{load_program, Machine, Snapshot, HaltReason, RunOutcome};
use std::io::{Read, Write};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut program: Option<String> = None;
    let mut max_steps: u64 = 100_000_000;
    let mut snapshot_out: Option<String> = None;
    let mut resume: Option<String> = None;
    let mut json_report = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--snapshot-out" => snapshot_out = args.next(),
            "--resume" => resume = args.next(),
            "--json" => json_report = true,
            s if !s.starts_with("--") && program.is_none() => program = Some(s.to_string()),
            s if !s.starts_with("--") && program.is_some() => {
                if let Ok(v) = s.parse::<u64>() {
                    if v > 0 { max_steps = v; }
                }
            }
            _ => {}
        }
    }

    let mut machine = if let Some(resume_path) = resume {
        let txt = std::fs::read_to_string(&resume_path)
            .unwrap_or_else(|e| { eprintln!("error: cannot read snapshot {resume_path}: {e}"); std::process::exit(2); });
        let snap: Snapshot = serde_json::from_str(&txt)
            .unwrap_or_else(|e| { eprintln!("error: bad snapshot: {e}"); std::process::exit(2); });
        Machine::from_snapshot(&snap)
    } else {
        let path = program.unwrap_or_else(|| {
            eprintln!("usage: rustbolge <program.mal> [max_steps] [--snapshot-out f] [--resume f] [--json] < input");
            std::process::exit(1);
        });
        let src = std::fs::read(&path)
            .unwrap_or_else(|e| { eprintln!("error: cannot open {path}: {e}"); std::process::exit(2); });
        let cells = match load_program(&src) {
            Ok(c) => c,
            Err(bad) => {
                eprintln!("error: invalid character 0x{bad:02x} in program");
                std::process::exit(2);
            }
        };
        Machine::new(&cells)
    };

    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).ok();

    let outcome: RunOutcome = machine.run(&input, max_steps);

    std::io::stdout().write_all(&outcome.output).ok();
    if !outcome.output.is_empty() {
        println!();
    }

    let status = match outcome.halt_reason {
        HaltReason::StepsExhausted => "TIMEOUT",
        _ => "HALTED",
    };
    if json_report {
        eprintln!(
            "{}",
            serde_json::json!({
                "steps": machine.steps,
                "status": status,
                "halt_reason": format!("{:?}", outcome.halt_reason),
                "final": {"a": outcome.final_a, "c": outcome.final_c, "d": outcome.final_d},
                "output_len": outcome.output.len(),
            })
        );
    } else {
        eprintln!("steps: {} | status: {}", machine.steps, status);
    }

    if let Some(path) = snapshot_out {
        let snap = machine.snapshot();
        let txt = serde_json::to_string(&snap).unwrap();
        std::fs::write(&path, txt)
            .unwrap_or_else(|e| { eprintln!("error: cannot write snapshot {path}: {e}"); std::process::exit(2); });
    }
}
