//! Lists 1 and 2 (Steps 8 through 11): the present game only.

use crate::board::*;
use crate::exchange::{self, Exchange};
use crate::strategies::{self, Ctx, TurnInfo, NAMES};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List1Row {
    pub id: usize,
    pub name: &'static str,
    pub flags: u8,
    pub score: u32,
}

/// List 1: the 20 templates applied to the present board and the five-turn window.
pub fn list1(b: &Board, white: bool, window: &[TurnInfo]) -> Vec<List1Row> {
    let ctx = Ctx { b, white, window };
    let mut rows: Vec<List1Row> = strategies::read_all(&ctx)
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let f = r.flags();
            let score = strategies::score_now(f);
            if score == 0 {
                None
            } else {
                Some(List1Row { id: i, name: NAMES[i], flags: f, score })
            }
        })
        .collect();
    sort_rows(&mut rows);
    rows
}

pub fn sort_rows(rows: &mut [List1Row]) {
    rows.sort_by(|a, b| {
        strategies::stage_rank(a.score)
            .cmp(&strategies::stage_rank(b.score))
            .then(b.score.cmp(&a.score))
            .then(a.name.cmp(b.name))
    });
}

#[derive(Clone, Debug)]
pub struct List2Row {
    pub strategy: &'static str,
    pub mv: Move,
    pub san: String,
    pub piece: i8,
    pub victim: i8,
    pub from: u8,
    pub victim_sq: u8,
    pub exchange: Exchange,
}

/// List 2: legal captures tied to the List 1 strategies (Step 14.3 rule 8),
/// only when it is the evaluated player's turn.
pub fn list2(b: &Board, white: bool, window: &[TurnInfo]) -> Vec<List2Row> {
    if b.white_to_move != white {
        return Vec::new();
    }
    let ctx = Ctx { b, white, window };
    let readings = strategies::read_all(&ctx);
    let captures: Vec<Move> = b.legal_moves().into_iter().filter(|m| b.is_capture(m)).collect();
    let mut rows = Vec::new();
    for (i, r) in readings.iter().enumerate() {
        if strategies::score_now(r.flags()) == 0 {
            continue;
        }
        for m in &captures {
            let vs = b.captured_sq(m);
            if b.sq[vs as usize].abs() == KING {
                continue;
            }
            if r.participants.contains(&m.from) && r.targets.contains(&vs) {
                rows.push(List2Row {
                    strategy: NAMES[i],
                    mv: *m,
                    san: b.san(m),
                    piece: b.sq[m.from as usize],
                    victim: b.captured_piece(m),
                    from: m.from,
                    victim_sq: vs,
                    exchange: exchange::evaluate(b, m),
                });
            }
        }
    }
    rows.sort_by(|a, b| {
        b.exchange
            .net()
            .cmp(&a.exchange.net())
            .then(b.exchange.gain.cmp(&a.exchange.gain))
            .then(a.exchange.loss.cmp(&b.exchange.loss))
            .then(a.strategy.cmp(b.strategy))
            .then(a.san.cmp(&b.san))
            .then(a.victim_sq.cmp(&b.victim_sq))
    });
    rows
}
