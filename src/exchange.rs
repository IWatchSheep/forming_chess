//! Capture valuation (Step 14.3), used by List 2.

use crate::board::*;

pub const MATE_BONUS: i32 = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exchange {
    pub gain: i32,
    pub loss: i32,
}

impl Exchange {
    pub fn net(&self) -> i32 {
        self.gain - self.loss
    }
}

/// Value of one step: the piece taken, plus a promotion bonus, plus a mate bonus.
fn step_value(b: &Board, m: &Move) -> (i32, bool) {
    let mut v = if b.is_en_passant(m) { 100 } else { piece_value(b.captured_piece(m)) };
    if m.promo != 0 {
        v += piece_value(m.promo) - 100;
    }
    let after = b.make(m);
    let mate = after.is_checkmate();
    if mate {
        v += MATE_BONUS;
    }
    (v, mate)
}

/// The cheapest legal recapture on `target`. A promoting pawn counts as a pawn,
/// and promotes to a queen.
fn cheapest_recapture(b: &Board, target: u8) -> Option<Move> {
    let mut best: Option<(i32, Move)> = None;
    for m in b.legal_moves() {
        if m.to != target || !b.is_capture(&m) || (m.promo != 0 && m.promo != QUEEN) {
            continue;
        }
        let p = b.sq[m.from as usize];
        let order = if p.abs() == KING { 10_000 } else { piece_value(p) };
        if best.is_none_or(|(o, _)| order < o) {
            best = Some((order, m));
        }
    }
    best.map(|(_, m)| m)
}

/// Evaluate playing `m` and the exchange that follows on its landing square.
/// The first move is always played; each later recapture may be declined, and
/// an exchange stops when continuing would give an equal net value.
pub fn evaluate(b: &Board, m: &Move) -> Exchange {
    let mut values: Vec<i32> = Vec::new();
    let (v0, mate0) = step_value(b, m);
    values.push(v0);
    let mut cur = b.make(m);
    let target = m.to;
    if !mate0 {
        for _ in 0..32 {
            let Some(r) = cheapest_recapture(&cur, target) else { break };
            let (v, mate) = step_value(&cur, &r);
            values.push(v);
            cur = cur.make(&r);
            if mate {
                break;
            }
        }
    }
    // best[k]: what the side making step k nets by playing it and then playing on well.
    let n = values.len();
    let mut best = vec![0i32; n + 1];
    for k in (0..n).rev() {
        let next = if k + 1 < n { best[k + 1] } else { 0 };
        best[k] = values[k] - next.max(0);
    }
    let mut gain = 0;
    let mut loss = 0;
    for k in 0..n {
        if k > 0 && best[k] <= 0 {
            break;
        }
        if k % 2 == 0 {
            gain += values[k];
        } else {
            loss += values[k];
        }
    }
    Exchange { gain, loss }
}
