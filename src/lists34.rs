//! Lists 3 and 4 (Steps 12 and 13): made from the position index, never by looking ahead.

use crate::board::*;
use crate::index::{Counts, Index, Status};
use crate::normalize::{self, Cat};
use crate::strategies::{self, F_COMPLETION, F_FOUNDATION, F_PROGRESS, NAMES};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// Proven win for the pawn side, in this many individual moves counting this move.
    ProvenWin(u32),
    ProvenDraw,
    Tried,
    /// Never tried here; counts come from similar positions (king and pawn against king only).
    Similar,
    NeverTried,
}

#[derive(Clone, Debug)]
pub struct List3Row {
    pub mv: Move,
    pub san: String,
    pub kind: RowKind,
    pub counts: Counts,
    /// Strategy notes at the destination: (name, score 30/45/60/75/100).
    pub notes: Vec<(&'static str, u32)>,
    /// Position in the Step 4 listing order.
    pub order: usize,
}

#[derive(Clone, Debug)]
pub struct List3 {
    pub rows: Vec<List3Row>,
    pub cat: Cat,
    pub pawn_side_to_move: bool,
    pub position_known: bool,
}

/// The List 3 score of one strategy from played moves (Step 14.1 rule 8).
pub fn played_score(d: u8, f: u8) -> u32 {
    if d & F_COMPLETION != 0 {
        100
    } else if d & F_FOUNDATION != 0 && d & F_PROGRESS != 0 {
        if f & F_COMPLETION != 0 {
            75
        } else {
            60
        }
    } else if d & F_FOUNDATION != 0 {
        if f & (F_PROGRESS | F_COMPLETION) != 0 {
            45
        } else {
            30
        }
    } else if d & F_PROGRESS != 0 {
        30
    } else {
        0
    }
}

/// The highest score that any one game's pair earns (Step 12): pairs are never split.
pub fn best_played_score(mask: u64) -> u32 {
    (0..64u32).filter(|b| mask >> b & 1 == 1).map(|b| played_score((b / 8) as u8, (b % 8) as u8)).max().unwrap_or(0)
}

pub fn list3(ix: &Index, b: &Board, bank: Option<&str>) -> List3 {
    let n = normalize::normalize(b);
    let node = ix.lookup(&n.key());
    let family = n.cat.proof_family();
    let pawn_side_to_move = family && n.board.white_to_move;
    let sim_key = normalize::similar_key(&n);
    let mut rows = Vec::new();
    for (order, m) in b.legal_moves().iter().enumerate() {
        let nm = n.t.mv(m);
        let nu = nm.uci();
        let san = b.san(m);
        let edge = node.and_then(|id| {
            let nd = &ix.nodes[id];
            nd.legal.iter().position(|x| *x == nu).and_then(|i| nd.edges[i])
        });
        let row = match edge {
            Some(e) => {
                let counts = ix.edge_counts(e, bank);
                let kind = if family {
                    match ix.edge_value(e) {
                        Status::Win(d) => RowKind::ProvenWin(d + 1),
                        Status::Draw => RowKind::ProvenDraw,
                        Status::Unknown => RowKind::Tried,
                    }
                } else {
                    RowKind::Tried
                };
                let ed = &ix.edges[e];
                let mut notes: Vec<(&'static str, u32)> = (0..20)
                    .filter_map(|k| {
                        let s = best_played_score(ed.pairs[k]);
                        if s > 0 {
                            Some((NAMES[k], s))
                        } else {
                            None
                        }
                    })
                    .collect();
                notes.sort_by(|a, b| {
                    strategies::stage_rank(a.1).cmp(&strategies::stage_rank(b.1)).then(b.1.cmp(&a.1)).then(a.0.cmp(b.0))
                });
                List3Row { mv: *m, san, kind, counts, notes, order }
            }
            None => {
                let mut counts = Counts::default();
                let mut kind = RowKind::NeverTried;
                if let Some(sk) = &sim_key {
                    let rm = normalize::relative_move(&n.board, &nm);
                    if let Some(banks) = ix.similar.get(&format!("{}|{}", sk, rm)) {
                        for (bk, c) in banks {
                            if bank.is_none_or(|w| w == bk) {
                                counts.add(c);
                            }
                        }
                        if counts.played > 0 {
                            kind = RowKind::Similar;
                        }
                    }
                }
                List3Row { mv: *m, san, kind, counts, notes: Vec::new(), order }
            }
        };
        rows.push(row);
    }
    sort_list3(&mut rows, family, pawn_side_to_move);
    List3 { rows, cat: n.cat, pawn_side_to_move, position_known: node.is_some() }
}

fn group(kind: &RowKind, family: bool, pawn_side: bool) -> u8 {
    let never = matches!(kind, RowKind::NeverTried | RowKind::Similar);
    if !family {
        return if never { 1 } else { 0 };
    }
    if pawn_side {
        match kind {
            RowKind::ProvenWin(_) => 0,
            RowKind::Tried => 1,
            RowKind::ProvenDraw => 3,
            _ => 2,
        }
    } else {
        match kind {
            RowKind::ProvenDraw => 0,
            RowKind::Tried => 2,
            RowKind::ProvenWin(_) => 3,
            _ => 1,
        }
    }
}

fn sort_list3(rows: &mut [List3Row], family: bool, pawn_side: bool) {
    rows.sort_by(|a, b| {
        let ga = group(&a.kind, family, pawn_side);
        let gb = group(&b.kind, family, pawn_side);
        ga.cmp(&gb).then_with(|| match (&a.kind, &b.kind) {
            (RowKind::ProvenWin(x), RowKind::ProvenWin(y)) => {
                if pawn_side {
                    x.cmp(y)
                } else {
                    y.cmp(x)
                }
            }
            (RowKind::Tried, RowKind::Tried) => {
                let (ra, rb) = (a.counts.win_rate(), b.counts.win_rate());
                // The pawn side (or the side to move) wants a high rate; the lone king a low one.
                let ord = if !family || pawn_side { rb.partial_cmp(&ra) } else { ra.partial_cmp(&rb) };
                ord.unwrap_or(std::cmp::Ordering::Equal)
            }
            _ => std::cmp::Ordering::Equal,
        })
        .then(a.order.cmp(&b.order))
    });
}

#[derive(Clone, Debug)]
pub struct List4Row {
    pub mv: Move,
    pub san: String,
    pub reply: String,
    pub next: String,
    pub count: u64,
    pub gain: i32,
    pub loss: i32,
    pub bank: String,
}

impl List4Row {
    pub fn net(&self) -> i32 {
        self.gain - self.loss
    }
}

/// List 4: captures and promotions that past games actually played, in the
/// move and the full turn after it. Each different continuation is one row.
pub fn list4(ix: &Index, b: &Board, bank: Option<&str>) -> Vec<List4Row> {
    let n = normalize::normalize(b);
    let Some(id) = ix.lookup(&n.key()) else { return Vec::new() };
    let node = &ix.nodes[id];
    let mut rows = Vec::new();
    for m in b.legal_moves() {
        let nu = n.t.mv(&m).uci();
        let Some(i) = node.legal.iter().position(|x| *x == nu) else { continue };
        let Some(e) = node.edges[i] else { continue };
        let after = b.make(&m);
        let mut merged: std::collections::BTreeMap<(String, String), (u64, i32, i32, String)> = Default::default();
        for ((bk, r, nx), c) in &ix.edges[e].conts {
            if !bank.is_none_or(|w| w == bk) {
                continue;
            }
            let reply_real = if r == "-" { None } else { after.parse_move(&n.t.move_text(r)) };
            let reply_san = reply_real.map(|x| after.san(&x)).unwrap_or_else(|| "-".into());
            let next_san = match reply_real {
                Some(rm) if nx != "-" => {
                    let a2 = after.make(&rm);
                    a2.parse_move(&n.t.move_text(nx)).map(|x| a2.san(&x)).unwrap_or_else(|| "-".into())
                }
                _ => "-".into(),
            };
            let entry = merged.entry((reply_san, next_san)).or_insert((0, c.gain, c.loss, bk.clone()));
            entry.0 += c.count;
            if entry.3 != *bk {
                entry.3 = "all".into();
            }
        }
        for ((reply, next), (count, gain, loss, bk)) in merged {
            rows.push(List4Row { mv: m, san: b.san(&m), reply, next, count, gain, loss, bank: bk });
        }
    }
    rows.sort_by(|a, b| {
        b.net()
            .cmp(&a.net())
            .then(b.gain.cmp(&a.gain))
            .then(a.loss.cmp(&b.loss))
            .then(a.san.cmp(&b.san))
            .then(a.reply.cmp(&b.reply))
            .then(a.next.cmp(&b.next))
    });
    rows
}
