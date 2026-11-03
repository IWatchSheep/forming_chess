//! Step 15.6: the drill. Plays both sides of seeded king-and-pawn starts and
//! stores the games in the drill bank.

use crate::archive::{Archive, DRILL};
use crate::board::*;
use crate::index::{goal_after, Counts, GameRecord, GameResult, Index, SavedMove, Status};
use crate::normalize;
use crate::strategies::{self, Ctx, TurnInfo};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A small seeded generator (SplitMix64), so a seed always gives the same order.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// The drill's starts: the normalized king-and-pawn set, seventh rank first,
/// then sixth, down to the second. The seed sets the order within a rank.
pub fn start_sequence(seed: u64) -> Vec<Board> {
    let all = normalize::kpk_positions(true);
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(all.len());
    for rank in (1..=6).rev() {
        let mut group: Vec<Board> = all
            .iter()
            .copied()
            .filter(|b| (0..64u8).any(|s| b.sq[s as usize] == PAWN && rank_of(s) == rank))
            .collect();
        for i in (1..group.len()).rev() {
            let j = rng.below(i + 1);
            group.swap(i, j);
        }
        out.extend(group);
    }
    out
}

/// The drill's move choice (Step 15.6).
pub fn choose(ix: &Index, b: &Board) -> Option<Move> {
    let n = normalize::normalize(b);
    let nlegal = n.board.legal_moves();
    if nlegal.is_empty() {
        return None;
    }
    let node = ix.lookup(&n.key());
    let vals: Vec<Option<(Status, Counts)>> = nlegal
        .iter()
        .map(|m| {
            let id = node?;
            let nd = &ix.nodes[id];
            let i = nd.legal.iter().position(|x| *x == m.uci())?;
            let e = nd.edges[i]?;
            Some((ix.edge_value(e), ix.edge_counts(e, None)))
        })
        .collect();
    let pick = |i: usize| Some(n.t.mv(&nlegal[i]));
    let first_untried = vals.iter().position(|v| v.is_none());
    let pawn_side = n.cat.proof_family() && n.board.white_to_move;
    let rate = |i: usize| vals[i].map(|(_, c)| c.win_rate()).unwrap_or(0.0);
    if !n.cat.proof_family() || pawn_side {
        // 1. the shortest proven win
        let best = (0..vals.len())
            .filter_map(|i| match vals[i] {
                Some((Status::Win(d), _)) => Some((d, i)),
                _ => None,
            })
            .min();
        if let Some((_, i)) = best {
            return pick(i);
        }
        // 2. the first never-tried move
        if let Some(i) = first_untried {
            return pick(i);
        }
        // 3. the best win rate for the pawn side
        let mut bi = 0;
        for i in 1..vals.len() {
            if rate(i) > rate(bi) {
                bi = i;
            }
        }
        pick(bi)
    } else {
        // 1. a proven draw
        if let Some(i) = vals.iter().position(|v| matches!(v, Some((Status::Draw, _)))) {
            return pick(i);
        }
        // 2. the first never-tried move
        if let Some(i) = first_untried {
            return pick(i);
        }
        // 3. all tried and all proven losses: the longest one
        if vals.iter().all(|v| matches!(v, Some((Status::Win(_), _)))) {
            let worst = (0..vals.len())
                .filter_map(|i| match vals[i] {
                    Some((Status::Win(d), _)) => Some((std::cmp::Reverse(d), i)),
                    _ => None,
                })
                .min();
            if let Some((_, i)) = worst {
                return pick(i);
            }
        }
        // 4. the lowest win rate for the pawn side
        let mut bi = 0;
        for i in 1..vals.len() {
            if rate(i) < rate(bi) {
                bi = i;
            }
        }
        pick(bi)
    }
}

/// The mover's template flags after a move, with that side's window.
pub fn mover_flags(after: &Board, white: bool, window: &[TurnInfo]) -> [u8; 20] {
    strategies::flags_all(&Ctx { b: after, white, window })
}

pub struct PlayedGame {
    pub moves: Vec<(Board, Move, [u8; 20])>,
    pub result: GameResult,
}

/// Play one drill game from `start`, both sides choosing by Step 15.6.
pub fn play(ix: &Index, start: &Board) -> PlayedGame {
    let mut b = *start;
    let mut seen: HashMap<String, u32> = HashMap::new();
    seen.insert(b.key(), 1);
    let mut quiet = 0u32;
    let mut windows: [Vec<TurnInfo>; 2] = [Vec::new(), Vec::new()];
    let mut moves = Vec::new();
    loop {
        let Some(m) = choose(ix, &b) else {
            return PlayedGame { moves, result: GameResult::Draw };
        };
        let after = b.make(&m);
        let white = b.white_to_move;
        let w = &mut windows[if white { 0 } else { 1 }];
        w.push(TurnInfo { white, from: m.from, to: m.to, piece: b.sq[m.from as usize], captured: b.captured_piece(&m) });
        if w.len() > crate::history::WINDOW {
            w.remove(0);
        }
        let flags = mover_flags(&after, white, w);
        let pawn_or_capture = b.sq[m.from as usize].abs() == PAWN || b.is_capture(&m);
        moves.push((b, m, flags));
        if let Some(r) = goal_after(&b, &m, &after) {
            return PlayedGame { moves, result: r };
        }
        let c = seen.entry(after.key()).or_insert(0);
        *c += 1;
        if *c >= 3 {
            return PlayedGame { moves, result: GameResult::DrawByRule };
        }
        quiet = if pawn_or_capture { 0 } else { quiet + 1 };
        if quiet >= 100 {
            return PlayedGame { moves, result: GameResult::DrawByRule };
        }
        b = after;
    }
}

#[derive(Debug, Default, Clone)]
pub struct DrillReport {
    pub games: u64,
    pub pawn_wins: u64,
    pub draws: u64,
    pub draws_by_rule: u64,
    pub first_start: Option<String>,
}

fn read_progress(path: &Path) -> HashMap<u64, usize> {
    let mut m = HashMap::new();
    if let Ok(t) = fs::read_to_string(path) {
        for l in t.lines() {
            let mut it = l.split_whitespace();
            if let (Some(a), Some(b)) = (it.next(), it.next()) {
                if let (Ok(s), Ok(p)) = (a.parse(), b.parse()) {
                    m.insert(s, p);
                }
            }
        }
    }
    m
}

fn write_progress(path: &Path, m: &HashMap<u64, usize>) {
    let mut keys: Vec<&u64> = m.keys().collect();
    keys.sort();
    let text: String = keys.iter().map(|k| format!("{} {}\n", k, m[k])).collect();
    let _ = fs::write(path, text);
}

/// Run `games` drill games for `seed`, continuing where that seed stopped last time.
pub fn run(
    ix: &mut Index,
    archive: &Archive,
    seed: u64,
    games: u64,
    mut on_progress: impl FnMut(u64, &Index),
) -> Result<DrillReport, String> {
    let seq = start_sequence(seed);
    let progress_path = archive.root.join("drill_progress.txt");
    let mut progress = read_progress(&progress_path);
    let mut pos = progress.get(&seed).copied().unwrap_or(0);
    let run_file = format!("drill_run_{}.txt", archive.next_id());
    let mut rep = DrillReport::default();
    for g in 0..games {
        let start = seq[pos % seq.len()];
        pos += 1;
        if rep.first_start.is_none() {
            rep.first_start = Some(start.key());
        }
        let played = play(ix, &start);
        let mut w = archive.begin_game(DRILL, &start, Some(&run_file)).map_err(|e| e.to_string())?;
        for (i, (b, m, f)) in played.moves.iter().enumerate() {
            w.add_move(i as u32 + 1, b, m, f).map_err(|e| e.to_string())?;
        }
        let id = w.id;
        w.finish(played.result).map_err(|e| e.to_string())?;
        let rec = GameRecord {
            id,
            bank: DRILL.to_string(),
            start,
            moves: played.moves.iter().map(|(_, m, f)| SavedMove { uci: m.uci(), flags: *f }).collect(),
            result: played.result,
        };
        ix.merge(&rec)?;
        rep.games += 1;
        match played.result {
            GameResult::WhiteWin | GameResult::BlackWin => rep.pawn_wins += 1,
            GameResult::DrawByRule => rep.draws_by_rule += 1,
            _ => rep.draws += 1,
        }
        if (g + 1) % 10_000 == 0 {
            ix.save(&archive.index_path()).map_err(|e| e.to_string())?;
            progress.insert(seed, pos);
            write_progress(&progress_path, &progress);
        }
        on_progress(g + 1, ix);
    }
    ix.save(&archive.index_path()).map_err(|e| e.to_string())?;
    progress.insert(seed, pos);
    write_progress(&progress_path, &progress);
    Ok(rep)
}
