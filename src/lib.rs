//! Rustbolge — Classic Malbolge engine (59049 ternary cells) ported from
//! Malbolge-Engine `vm.c`, plus the capability the genealogy marks
//! NOT_DEMONSTRATED: full snapshot / resume with JSON serialization.
//!
//! Semantics are bit-exact with `Malbolge-Engine/src/vm.c`:
//! crazy5 split-chunk table, lazy crazy chain fill with block granularity,
//! write-through shadow overlay, and the same instruction decoding
//! `(mem[c] + c) % 94`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MEM_SIZE: u32 = 59049;
pub const LAST: u32 = MEM_SIZE - 1;
pub const POW9: u32 = 19683;
pub const HALF: u32 = 243;
pub const BLOCK_SIZE: u32 = 243;
pub const OUT_CAP: usize = 65536;

pub const ENCRYPT: &[u8; 94] =
    b"5z]&gqtyfr$(we4{WP)H-Zn,[%\\3dL+Q;>U!pJS72FhOA1CB6v^=I_0/8|jsb9m<.TVac`uY*MK'X~xDl}REokN:#?G\"i@";

const CRAZY_TBL: [[u8; 3]; 3] = [[1, 0, 0], [1, 0, 2], [2, 2, 1]];

fn build_crazy5() -> Vec<Vec<u32>> {
    let mut tbl = vec![vec![0u32; HALF as usize]; HALF as usize];
    for a5 in 0..HALF as usize {
        for b5 in 0..HALF as usize {
            let (mut r, mut p) = (0u32, 1u32);
            let (mut aa, mut bb) = (a5 as u32, b5 as u32);
            for _ in 0..5 {
                r += CRAZY_TBL[(bb % 3) as usize][(aa % 3) as usize] as u32 * p;
                aa /= 3;
                bb /= 3;
                p *= 3;
            }
            tbl[a5][b5] = r;
        }
    }
    tbl
}

pub fn crazy(a: u32, b: u32, crazy5: &[Vec<u32>]) -> u32 {
    crazy5[(a % HALF) as usize][(b % HALF) as usize]
        + HALF * crazy5[(a / HALF) as usize][(b / HALF) as usize]
}

pub fn rotate(n: u32) -> u32 {
    POW9 * (n % 3) + n / 3
}

/// Full VM state, serializable. This is the snapshot that Autobolge-Zig
/// doesn't have in the genealogy matrix.
#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub a: u32,
    pub c: u32,
    pub d: u32,
    pub halted: bool,
    pub steps: u64,
    pub input_pos: u64,
    pub output: Vec<u8>,
    pub fill_start: u32,
    pub chain_until: u32,
    /// chain cells computed so far: [0, chain_until)
    pub chain: Vec<u32>,
    /// overlay writes, sorted by address for deterministic hashes
    pub overlay: Vec<(u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub steps: u64,
    pub terminated: bool,
    pub halt_reason: HaltReason,
    pub output: Vec<u8>,
    pub final_a: u32,
    pub final_c: u32,
    pub final_d: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaltReason {
    /// instruction 81 (`v`) executed
    VInstruction,
    /// mem[c] was outside printable range
    InvalidCell,
    /// input read past end of input (EOF behavior: vm.c halts)
    Eof,
    /// max_steps reached without termination
    StepsExhausted,
    /// never ran (restored from a halted snapshot)
    RestoredHalted,
}

pub struct Machine {
    pub a: u32,
    pub c: u32,
    pub d: u32,
    pub halted: bool,
    pub halt_reason: Option<HaltReason>,
    pub steps: u64,
    pub input_pos: usize,
    pub output: Vec<u8>,
    fill_start: u32,
    chain_until: u32,
    chain: Vec<u32>,
    overlay: HashMap<u32, u32>,
    crazy5: Vec<Vec<u32>>,
}

impl Machine {
    /// Load program cells (already filtered to 33..=126).
    pub fn new(cells: &[u32]) -> Self {
        let fill_start = if cells.len() > 2 { cells.len() as u32 } else { 2 };
        let mut m = Machine {
            a: 0,
            c: 0,
            d: 0,
            halted: false,
            halt_reason: None,
            steps: 0,
            input_pos: 0,
            output: Vec::new(),
            fill_start,
            chain_until: fill_start,
            chain: vec![0u32; MEM_SIZE as usize],
            overlay: HashMap::with_capacity(cells.len()),
            crazy5: build_crazy5(),
        };
        for (i, &cell) in cells.iter().enumerate() {
            m.overlay.insert(i as u32, cell);
        }
        m
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut overlay: Vec<(u32, u32)> = self.overlay.iter().map(|(&k, &v)| (k, v)).collect();
        overlay.sort_unstable();
        Snapshot {
            a: self.a,
            c: self.c,
            d: self.d,
            halted: self.halted,
            steps: self.steps,
            input_pos: self.input_pos as u64,
            output: self.output.clone(),
            fill_start: self.fill_start,
            chain_until: self.chain_until,
            chain: self.chain[..self.chain_until as usize].to_vec(),
            overlay,
        }
    }

    pub fn from_snapshot(s: &Snapshot) -> Self {
        let mut chain = vec![0u32; MEM_SIZE as usize];
        chain[..s.chain.len()].copy_from_slice(&s.chain);
        let mut m = Machine {
            a: s.a,
            c: s.c,
            d: s.d,
            halted: s.halted,
            halt_reason: if s.halted { Some(HaltReason::RestoredHalted) } else { None },
            steps: s.steps,
            input_pos: s.input_pos as usize,
            output: s.output.clone(),
            fill_start: s.fill_start,
            chain_until: s.chain_until,
            chain,
            overlay: s.overlay.iter().copied().collect(),
            crazy5: build_crazy5(),
        };
        // a restored halted machine reports itself correctly
        if m.halted {
            m.halt_reason = Some(HaltReason::RestoredHalted);
        }
        m
    }

    fn mem_get(&mut self, x: u32) -> u32 {
        if let Some(&v) = self.overlay.get(&x) {
            return v;
        }
        if x < self.fill_start {
            return 0;
        }
        if x >= self.chain_until {
            self.ensure_filled(x);
        }
        self.chain[x as usize]
    }

    fn mem_set(&mut self, x: u32, v: u32) {
        self.overlay.insert(x, v);
    }

    fn ensure_filled(&mut self, x: u32) {
        let block_end = (x / BLOCK_SIZE + 1) * BLOCK_SIZE;
        let tail_end = (self.fill_start / BLOCK_SIZE + 1) * BLOCK_SIZE;

        let (mut i, mut p1, mut p2);
        if self.chain_until == self.fill_start {
            p1 = self.overlay.get(&(self.fill_start - 1)).copied().unwrap_or(0);
            p2 = self.overlay.get(&(self.fill_start - 2)).copied().unwrap_or(0);
            i = self.fill_start;
        } else {
            i = self.chain_until;
            p1 = self.chain[(i - 1) as usize];
            p2 = self.chain[(i - 2) as usize];
        }

        while i < block_end {
            let nxt = if i == tail_end {
                let (v1, v2) = (self.mem_get(i - 1), self.mem_get(i - 2));
                crazy(v1, v2, &self.crazy5)
            } else if i == tail_end + 1 {
                let v2 = self.mem_get(i - 2);
                crazy(p1, v2, &self.crazy5)
            } else {
                crazy(p1, p2, &self.crazy5)
            };
            self.chain[i as usize] = nxt;
            p2 = p1;
            p1 = nxt;
            i += 1;
        }
        self.chain_until = block_end;
    }

    /// Run up to `max_steps` more instructions with the given input bytes.
    pub fn run(&mut self, input: &[u8], max_steps: u64) -> RunOutcome {
        let mut executed: u64 = 0;
        while !self.halted && executed < max_steps {
            executed += 1;
            let ins = self.mem_get(self.c);
            if !(33..=126).contains(&ins) {
                self.halted = true;
                self.halt_reason = Some(HaltReason::InvalidCell);
                break;
            }
            let v = (ins + self.c) % 94;
            match v {
                4 => {
                    self.c = self.mem_get(self.d);
                }
                5 => {
                    if self.output.len() < OUT_CAP {
                        self.output.push((self.a % 256) as u8);
                    }
                }
                23 => {
                    if self.input_pos < input.len() {
                        self.a = input[self.input_pos] as u32;
                        self.input_pos += 1;
                    } else {
                        self.halted = true;
                        self.halt_reason = Some(HaltReason::Eof);
                        break;
                    }
                }
                39 => {
                    let r = rotate(self.mem_get(self.d));
                    self.mem_set(self.d, r);
                    self.a = r;
                }
                40 => {
                    self.d = self.mem_get(self.d);
                }
                62 => {
                    let m = self.mem_get(self.d);
                    let r = crazy(self.a, m, &self.crazy5);
                    self.mem_set(self.d, r);
                    self.a = r;
                }
                81 => {
                    self.halted = true;
                    self.halt_reason = Some(HaltReason::VInstruction);
                    break;
                }
                _ => {}
            }
            let enc = self.mem_get(self.c);
            if (33..=126).contains(&enc) {
                self.mem_set(self.c, ENCRYPT[(enc - 33) as usize] as u32);
            }
            self.c = if self.c == LAST { 0 } else { self.c + 1 };
            self.d = if self.d == LAST { 0 } else { self.d + 1 };
        }
        self.steps = self.steps.saturating_add(executed);
        RunOutcome {
            steps: executed,
            terminated: self.halted,
            halt_reason: self
                .halt_reason
                .clone()
                .unwrap_or(HaltReason::StepsExhausted),
            output: self.output.clone(),
            final_a: self.a,
            final_c: self.c,
            final_d: self.d,
        }
    }
}

/// Loader compatible with malbolge.c: skip whitespace, reject non-printable.
pub fn load_program(source: &[u8]) -> Result<Vec<u32>, u8> {
    let mut cells = Vec::new();
    for &ch in source {
        match ch {
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            33..=126 => {
                if cells.len() < MEM_SIZE as usize {
                    cells.push(ch as u32);
                }
            }
            bad => return Err(bad),
        }
    }
    Ok(cells)
}

/// One-shot convenience: load + run.
pub fn run(cells: &[u32], input: &[u8], max_steps: u64) -> RunOutcome {
    let mut m = Machine::new(cells);
    m.run(input, max_steps)
}
