//! The position index, the proof rules, Lists 3 and 4, banks and the drill (the tests of Step 15.8).

use forming_chess::archive::{Archive, DRILL, GENERAL};
use forming_chess::board::*;
use forming_chess::drill;
use forming_chess::index::{GameRecord, GameResult, Index, SavedMove, Status};
use forming_chess::lists34::{list3, list4, RowKind};
use forming_chess::normalize::{self, Cat};
use std::path::PathBuf;

fn game(id: u64, bank: &str, fen: &str, moves: &[&str], result: GameResult) -> GameRecord {
    GameRecord {
        id,
        bank: bank.to_string(),
        start: Board::from_fen(fen).unwrap(),
        moves: moves.iter().map(|m| SavedMove { uci: m.to_string(), flags: [0; 20] }).collect(),
        result,
    }
}

fn status_of(ix: &Index, fen: &str) -> Status {
    let b = Board::from_fen(fen).unwrap();
    let id = ix.lookup(&normalize::normalize(&b).key()).expect("position stored");
    ix.nodes[id].status
}

fn temp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fc_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn empty_index_start_lists() {
    let ix = Index::new();
    let b = Board::start();
    let l3 = list3(&ix, &b, None);
    assert_eq!(l3.rows.len(), 20);
    assert!(l3.rows.iter().all(|r| r.kind == RowKind::NeverTried));
    assert!(list4(&ix, &b, None).is_empty());
}

#[test]
fn king_and_pawn_counts_and_mirror_share_an_entry() {
    assert_eq!(normalize::kpk_positions(false).len(), 331_352);
    assert_eq!(normalize::kpk_positions(true).len(), 165_676);
    let a = Board::from_fen("8/8/3k4/8/1P6/8/8/2K5 w - - 0 1").unwrap();
    let mirrored = Board::from_fen("8/8/4k3/8/6P1/8/8/5K2 w - - 0 1").unwrap();
    let black_pawn = Board::from_fen("2k5/8/8/1p6/8/3K4/8/8 b - - 0 1").unwrap();
    let ka = normalize::normalize(&a);
    assert_eq!(ka.key(), normalize::normalize(&mirrored).key());
    assert_eq!(ka.key(), normalize::normalize(&black_pawn).key());
    // a position and its mirror image list their moves in the same order
    assert_eq!(ka.board.legal_moves(), normalize::normalize(&mirrored).board.legal_moves());
}

#[test]
fn middlegame_key_is_exact_and_never_similar() {
    let b = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    let n = normalize::normalize(&b);
    assert_eq!(n.cat, Cat::Other);
    assert!(n.key().contains("|w|KQkq|-"));
    assert!(normalize::similar_key(&n).is_none());
}

#[test]
fn promotion_goals() {
    // a safe queen: win at distance 0, so the position before is a win in 1
    let fen = "8/P7/8/8/8/8/5k2/K7 w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["a7a8q"], GameResult::WhiteWin)).unwrap();
    assert_eq!(status_of(&ix, fen), Status::Win(1));
    // a rook that cannot be taken also wins; a bishop or knight is a draw
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["a7a8r"], GameResult::WhiteWin)).unwrap();
    assert_eq!(status_of(&ix, fen), Status::Win(1));
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["a7a8b"], GameResult::Draw)).unwrap();
    let id = ix.lookup(&normalize::normalize(&Board::from_fen(fen).unwrap()).key()).unwrap();
    let e = ix.nodes[id].edges.iter().flatten().next().copied().unwrap();
    assert_eq!(ix.edge_value(e), Status::Draw);
}

#[test]
fn stalemating_and_capturable_promotions_are_draws() {
    // Search the counted set for a queen promotion that stalemates, and one the king can take.
    let mut stalemate = None;
    let mut capturable = None;
    for b in normalize::kpk_positions(true) {
        if !b.white_to_move {
            continue;
        }
        for m in b.legal_moves().into_iter().filter(|m| m.promo == QUEEN) {
            let a = b.make(&m);
            if stalemate.is_none() && a.is_stalemate() {
                stalemate = Some((b, m));
            }
            if capturable.is_none() && a.legal_moves().iter().any(|r| r.to == m.to) {
                capturable = Some((b, m));
            }
        }
        if stalemate.is_some() && capturable.is_some() {
            break;
        }
    }
    for (b, m) in [stalemate.unwrap(), capturable.unwrap()] {
        let mut ix = Index::new();
        let fen = b.to_fen();
        ix.merge(&game(1, GENERAL, &fen, &[&m.uci()], GameResult::Draw)).unwrap();
        let id = ix.lookup(&normalize::normalize(&b).key()).unwrap();
        let e = ix.nodes[id].edges.iter().flatten().next().copied().unwrap();
        assert_eq!(ix.edge_value(e), Status::Draw, "{} {}", fen, m.uci());
    }
}

#[test]
fn captured_pawn_is_a_draw_and_proves_the_lone_king_position() {
    let fen = "8/8/8/8/8/1kP5/8/7K b - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["b3c3"], GameResult::Draw)).unwrap();
    assert_eq!(status_of(&ix, fen), Status::Draw);
}

#[test]
fn pawn_side_needs_one_win_lone_king_needs_every_move() {
    // Black to move; after any Black king move, White queens safely.
    let p0 = "8/P7/8/8/8/8/6k1/K7 b - - 0 1";
    let b0 = Board::from_fen(p0).unwrap();
    let black_moves = b0.legal_moves();
    assert!(black_moves.len() >= 3);
    let mut ix = Index::new();
    let first = black_moves[0].uci();
    ix.merge(&game(1, GENERAL, p0, &[&first, "a7a8q"], GameResult::WhiteWin)).unwrap();
    // the position after the Black move: one tried move into a win is enough
    let p1 = b0.make(&black_moves[0]);
    assert_eq!(status_of(&ix, &p1.to_fen()), Status::Win(1));
    // the lone king position is not proven while moves remain untried
    assert_eq!(status_of(&ix, p0), Status::Unknown);
    // the drill's lone king tries an untried move before the move known to lose
    let choice = drill::choose(&ix, &b0).unwrap();
    assert_ne!(choice, black_moves[0]);
    for (i, m) in black_moves.iter().enumerate().skip(1) {
        ix.merge(&game(i as u64 + 1, GENERAL, p0, &[&m.uci(), "a7a8q"], GameResult::WhiteWin)).unwrap();
    }
    assert_eq!(status_of(&ix, p0), Status::Win(2));
}

#[test]
fn proof_flows_back_to_positions_from_earlier_games() {
    // Game 1 reaches P2 through White's a6-a7 and stops there.
    let p1 = "8/8/P7/8/8/8/6k1/K7 w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, p1, &["a6a7"], GameResult::Stopped)).unwrap();
    assert_eq!(status_of(&ix, p1), Status::Unknown);
    // Later games, which never play P1, prove P2.
    let p2 = Board::from_fen(p1).unwrap().make(&Board::from_fen(p1).unwrap().parse_move("a6a7").unwrap());
    for (i, m) in p2.legal_moves().iter().enumerate() {
        ix.merge(&game(i as u64 + 2, GENERAL, &p2.to_fen(), &[&m.uci(), "a7a8q"], GameResult::WhiteWin)).unwrap();
    }
    assert_eq!(status_of(&ix, &p2.to_fen()), Status::Win(2));
    assert_eq!(status_of(&ix, p1), Status::Win(3));
}

#[test]
fn closing_rule_proves_a_circle_a_draw() {
    // Pawn side A and lone king B shuffle forever between two positions, every move tried.
    let mut ix = Index::new();
    let a = ix.add_test_node("A", Cat::Kpk, true, 1);
    let b = ix.add_test_node("B", Cat::Kpk, false, 1);
    ix.add_test_edge(a, 0, b, None);
    ix.add_test_edge(b, 0, a, None);
    ix.recheck(&[a, b]);
    assert_eq!(ix.nodes[a].status, Status::Draw);
    assert_eq!(ix.nodes[b].status, Status::Draw);
    // With an untried move left, the same circle is not closed.
    let mut ix = Index::new();
    let a = ix.add_test_node("A", Cat::Kpk, true, 2);
    let b = ix.add_test_node("B", Cat::Kpk, false, 1);
    ix.add_test_edge(a, 0, b, None);
    ix.add_test_edge(b, 0, a, None);
    ix.recheck(&[a, b]);
    assert_eq!(ix.nodes[a].status, Status::Unknown);
    // A circle with an exit to a proven win is not a draw.
    let mut ix = Index::new();
    let a = ix.add_test_node("A", Cat::Kpk, true, 2);
    let b = ix.add_test_node("B", Cat::Kpk, false, 1);
    let w = ix.add_test_node("W", Cat::Kpk, false, 0);
    ix.nodes[w].status = Status::Win(0);
    ix.add_test_edge(a, 0, b, None);
    ix.add_test_edge(a, 1, w, None);
    ix.add_test_edge(b, 0, a, None);
    ix.recheck(&[a, b]);
    assert_eq!(ix.nodes[a].status, Status::Win(1));
    assert_eq!(ix.nodes[b].status, Status::Win(2));
}

#[test]
fn draw_by_rule_counts_but_proves_nothing() {
    let fen = "8/8/8/8/8/1P6/7k/K7 w - - 0 1";
    let moves = ["a1b1", "h2h1", "b1a1", "h1h2", "a1b1", "h2h1", "b1a1", "h1h2"];
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &moves, GameResult::DrawByRule)).unwrap();
    assert!(ix.nodes.iter().all(|n| !n.status.is_proven()));
    let l = list3(&ix, &Board::from_fen(fen).unwrap(), None);
    let row = l.rows.iter().find(|r| r.mv.uci() == "a1b1").unwrap();
    assert_eq!(row.counts.draws, 2);
    assert_eq!(row.kind, RowKind::Tried);
}

#[test]
fn list3_order_for_the_pawn_side() {
    let fen = "8/P7/1k6/8/8/8/8/3K4 w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["a7a8q"], GameResult::WhiteWin)).unwrap();
    ix.merge(&game(2, GENERAL, fen, &["d1e1", "b6a7"], GameResult::Draw)).unwrap();
    ix.merge(&game(3, GENERAL, fen, &["d1c1"], GameResult::Stopped)).unwrap();
    let l = list3(&ix, &Board::from_fen(fen).unwrap(), None);
    assert!(l.pawn_side_to_move);
    assert_eq!(l.rows[0].mv.uci(), "a7a8q");
    assert_eq!(l.rows[0].kind, RowKind::ProvenWin(1));
    assert_eq!(l.rows[1].mv.uci(), "d1c1");
    assert_eq!(l.rows[1].kind, RowKind::Tried);
    assert_eq!(l.rows.last().unwrap().mv.uci(), "d1e1");
    assert_eq!(l.rows.last().unwrap().kind, RowKind::ProvenDraw);
    // the stopped game marks its move tried and adds no result
    assert_eq!(l.rows[1].counts.played, 1);
    assert_eq!(l.rows[1].counts.results(), 0);
}

#[test]
fn list3_order_for_the_lone_king_is_reversed() {
    let p0 = "8/P7/8/8/8/8/6k1/K7 b - - 0 1";
    let b0 = Board::from_fen(p0).unwrap();
    let first = b0.legal_moves()[0].uci();
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, p0, &[&first, "a7a8q"], GameResult::WhiteWin)).unwrap();
    let l = list3(&ix, &b0, None);
    assert!(!l.pawn_side_to_move);
    assert_eq!(l.rows.last().unwrap().mv.uci(), first);
    assert!(matches!(l.rows.last().unwrap().kind, RowKind::ProvenWin(_)));
    assert_eq!(l.rows[0].kind, RowKind::NeverTried);
}

#[test]
fn list4_shows_what_was_played() {
    // King and pawn: the defender takes the pawn after White's king move.
    let fen = "8/8/8/8/2k5/1P6/8/7K w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["h1g1", "c4b3"], GameResult::Draw)).unwrap();
    ix.merge(&game(2, GENERAL, fen, &["h1g1", "c4d3"], GameResult::Stopped)).unwrap();
    let rows = list4(&ix, &Board::from_fen(fen).unwrap(), None);
    assert_eq!(rows.len(), 1, "a continuation with no capture or promotion adds no row");
    assert_eq!((rows[0].gain, rows[0].loss, rows[0].net()), (0, 100, -100));
    // A promotion within the full turn is +800, and two continuations are two rows.
    let fen = "8/8/P7/8/8/8/6k1/K7 w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["a6a7", "g2g3", "a7a8q"], GameResult::WhiteWin)).unwrap();
    ix.merge(&game(2, GENERAL, fen, &["a6a7", "g2f3", "a7a8q"], GameResult::WhiteWin)).unwrap();
    ix.merge(&game(3, GENERAL, fen, &["a6a7", "g2f3", "a7a8q"], GameResult::WhiteWin)).unwrap();
    let rows = list4(&ix, &Board::from_fen(fen).unwrap(), None);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.gain == 800 && r.loss == 0));
    let counts: Vec<u64> = rows.iter().map(|r| r.count).collect();
    assert!(counts.contains(&1) && counts.contains(&2));
}

#[test]
fn similar_rows_are_labeled_and_never_used_as_proof() {
    let a = "8/8/8/8/2k5/8/2P5/6K1 w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, a, &["g1f1"], GameResult::Stopped)).unwrap();
    // the same shape one file to the right (still an inner-file pawn): king one step left is similar
    let b = Board::from_fen("8/8/8/8/3k4/8/3P4/7K w - - 0 1").unwrap();
    let l = list3(&ix, &b, None);
    let row = l.rows.iter().find(|r| r.mv.uci() == "h1g1").unwrap();
    assert_eq!(row.kind, RowKind::Similar);
    // the drill plays the first never-tried move, not the similar one
    let n = normalize::normalize(&b);
    let first = n.t.mv(&n.board.legal_moves()[0]);
    assert_eq!(drill::choose(&ix, &b), Some(first));
}

#[test]
fn bank_selection_changes_counts_not_proofs() {
    let fen = "8/P7/8/8/8/8/5k2/K7 w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["a7a8q"], GameResult::WhiteWin)).unwrap();
    ix.merge(&game(2, DRILL, fen, &["a7a8q"], GameResult::WhiteWin)).unwrap();
    ix.merge(&game(3, DRILL, fen, &["a7a8q"], GameResult::WhiteWin)).unwrap();
    let b = Board::from_fen(fen).unwrap();
    let all = list3(&ix, &b, None);
    let drill_only = list3(&ix, &b, Some(DRILL));
    let general = list3(&ix, &b, Some(GENERAL));
    assert_eq!(all.rows[0].counts.played, 3);
    assert_eq!(drill_only.rows[0].counts.played, 2);
    assert_eq!(general.rows[0].counts.played, 1);
    assert_eq!(all.rows[0].kind, RowKind::ProvenWin(1));
    assert_eq!(general.rows[0].kind, RowKind::ProvenWin(1));
}

#[test]
fn replaying_a_position_updates_one_entry() {
    let fen = "8/8/8/8/8/1P6/7k/K7 w - - 0 1";
    let mut ix = Index::new();
    ix.merge(&game(1, GENERAL, fen, &["a1b1"], GameResult::Stopped)).unwrap();
    let n = ix.nodes.len();
    ix.merge(&game(2, GENERAL, fen, &["a1b1"], GameResult::Stopped)).unwrap();
    assert_eq!(ix.nodes.len(), n);
    let l = list3(&ix, &Board::from_fen(fen).unwrap(), None);
    assert_eq!(l.rows.iter().find(|r| r.mv.uci() == "a1b1").unwrap().counts.played, 2);
}

#[test]
fn drill_starts_and_seed() {
    let s1 = drill::start_sequence(1);
    let again = drill::start_sequence(1);
    let s2 = drill::start_sequence(2);
    assert_eq!(s1.len(), 165_676);
    assert_eq!(s1[0], again[0]);
    assert_ne!(s1[..50].to_vec(), s2[..50].to_vec());
    let pawn_rank = |b: &Board| (0..64u8).find(|&s| b.sq[s as usize] == PAWN).map(rank_of).unwrap();
    let ranks: Vec<i8> = s1.iter().map(pawn_rank).collect();
    assert_eq!(ranks[0], 6);
    assert!(ranks.windows(2).all(|w| w[0] >= w[1]), "seventh rank first, down to the second");
    assert_eq!(*ranks.last().unwrap(), 1);
}

#[test]
fn drill_writes_compact_records_and_rebuilds_the_same_index() {
    let dir = temp_dir("drill");
    let ar = Archive::new(&dir);
    let mut ix = Index::new();
    let rep = drill::run(&mut ix, &ar, 7, 300, |_, _| {}).unwrap();
    assert_eq!(rep.games, 300);
    // no future-search file exists anywhere
    let mut files = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            if e.path().is_dir() {
                stack.push(e.path());
            } else {
                files.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    assert!(files.iter().all(|f| !f.contains("future")), "{:?}", files);
    assert_eq!(ar.bank_entries(DRILL).len(), 300);
    let saved = std::fs::read_to_string(ar.index_path()).unwrap();
    let rebuilt = ar.rebuild().unwrap();
    assert_eq!(rebuilt.to_text(), saved);
    let loaded = Index::load(&ar.index_path()).unwrap();
    assert_eq!(loaded.to_text(), saved);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn drill_proofs_are_correct_against_a_retrograde_solution() {
    // Plain retrograde solution of the normalized set under the goals of Step 12.
    let all = normalize::kpk_positions(true);
    let keys: Vec<String> = all.iter().map(|b| normalize::normalize(b).key()).collect();
    let id: std::collections::HashMap<&str, usize> = keys.iter().enumerate().map(|(i, k)| (k.as_str(), i)).collect();
    enum S {
        Goal(bool),
        Pos(usize),
    }
    let succ: Vec<Vec<S>> = all
        .iter()
        .map(|b| {
            b.legal_moves()
                .iter()
                .map(|m| {
                    let a = b.make(m);
                    if m.promo != 0 {
                        let r = a.legal_moves();
                        if r.is_empty() {
                            return S::Goal(a.in_check(a.white_to_move));
                        }
                        if m.promo == BISHOP || m.promo == KNIGHT {
                            return S::Goal(false);
                        }
                        return S::Goal(!r.iter().any(|x| x.to == m.to));
                    }
                    let na = normalize::normalize(&a);
                    if na.cat != Cat::Kpk {
                        return S::Goal(false);
                    }
                    S::Pos(id[na.key().as_str()])
                })
                .collect()
        })
        .collect();
    let mut win = vec![false; all.len()];
    for (i, b) in all.iter().enumerate() {
        if b.legal_moves().is_empty() && b.in_check(b.white_to_move) && !b.white_to_move {
            win[i] = true;
        }
    }
    loop {
        let mut changed = false;
        for i in 0..all.len() {
            if win[i] || succ[i].is_empty() {
                continue;
            }
            let v = |s: &S| match s {
                S::Goal(g) => *g,
                S::Pos(j) => win[*j],
            };
            let w = if all[i].white_to_move { succ[i].iter().any(v) } else { succ[i].iter().all(v) };
            if w {
                win[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    assert_eq!(win.iter().filter(|w| **w).count(), 111_282);
    let dir = temp_dir("verify");
    let ar = Archive::new(&dir);
    let mut ix = Index::new();
    drill::run(&mut ix, &ar, 1, 4000, |_, _| {}).unwrap();
    let mut proven = 0;
    for n in ix.nodes.iter().filter(|n| n.cat == Cat::Kpk) {
        let i = id[n.key.as_str()];
        match n.status {
            Status::Win(_) => {
                assert!(win[i], "false win {}", n.key);
                proven += 1;
            }
            Status::Draw => {
                assert!(!win[i], "false draw {}", n.key);
                proven += 1;
            }
            Status::Unknown => {}
        }
    }
    assert!(proven > 1000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stopped_and_killed_games() {
    let dir = temp_dir("stopped");
    let ar = Archive::new(&dir);
    let b = Board::start();
    let mut w = ar.begin_game(GENERAL, &b, None).unwrap();
    let m = b.parse_move("e2e4").unwrap();
    w.add_move(1, &b, &m, &[0; 20]).unwrap();
    let path = w.path.clone();
    drop(w); // the program was killed: no end block
    let before = std::fs::read_to_string(&path).unwrap();
    // Next start: marked stopped without changing earlier lines, then merged.
    let ar = Archive::new(&dir);
    assert_eq!(ar.mark_unfinished_stopped().unwrap(), vec![1]);
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.starts_with(&before));
    assert!(after.ends_with("GAME STOPPED\n"));
    let mut ix = Index::new();
    assert_eq!(ar.catch_up(&mut ix).unwrap(), 1);
    let l = list3(&ix, &b, None);
    let row = l.rows.iter().find(|r| r.mv.uci() == "e2e4").unwrap();
    assert_eq!(row.counts.played, 1);
    assert_eq!(row.counts.results(), 0);
    // A game in progress is not in the index.
    let ar = Archive::new(&dir);
    let mut w2 = ar.begin_game(GENERAL, &b, None).unwrap();
    w2.add_move(1, &b, &b.parse_move("d2d4").unwrap(), &[0; 20]).unwrap();
    let mut ix2 = Index::new();
    ar.catch_up(&mut ix2).unwrap();
    assert!(ix2.is_merged(1) && !ix2.is_merged(2));
    w2.finish(GameResult::Draw).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

fn flagged(id: u64, d: u8, later: u8) -> GameRecord {
    let mut g = game(id, GENERAL, START_FEN, &["e2e4", "e7e5", "g1f3"], GameResult::Draw);
    g.moves[0].flags[0] = d;
    g.moves[2].flags[0] = later;
    g
}

#[test]
fn scores_come_from_one_games_pair() {
    // Game A: foundation and progress at the destination, nothing after.
    // Game B: foundation only at the destination, then completion in its own full turn.
    // Joined, they would give 75. Neither game earned it: A gives 60, B gives 45.
    let mut ix = Index::new();
    ix.merge(&flagged(1, 3, 0)).unwrap();
    ix.merge(&flagged(2, 1, 4)).unwrap();
    let l = list3(&ix, &Board::start(), None);
    let row = l.rows.iter().find(|r| r.mv.uci() == "e2e4").unwrap();
    let scores: Vec<u32> = row.notes.iter().filter(|n| n.0 == forming_chess::strategies::NAMES[0]).map(|n| n.1).collect();
    assert_eq!(scores, vec![60]);
    // One game that earns 75 by itself still shows 75.
    ix.merge(&flagged(3, 3, 4)).unwrap();
    let l = list3(&ix, &Board::start(), None);
    let row = l.rows.iter().find(|r| r.mv.uci() == "e2e4").unwrap();
    assert!(row.notes.iter().any(|n| n.0 == forming_chess::strategies::NAMES[0] && n.1 == 75));
    // The pairs survive a save and a load.
    let again = Index::from_text(&ix.to_text()).unwrap();
    let l = list3(&again, &Board::start(), None);
    let row = l.rows.iter().find(|r| r.mv.uci() == "e2e4").unwrap();
    assert!(row.notes.iter().any(|n| n.0 == forming_chess::strategies::NAMES[0] && n.1 == 75));
}

#[test]
fn a_short_index_line_is_an_error_not_a_panic() {
    let ok = Index::new().to_text();
    assert!(Index::from_text(&ok).is_ok());
    for bad in ["K\t0\tdrill", "E\t0\t1", "N\tkey", "C\t0\tb\t1", "P\t0", "S\ta\tb", "G"] {
        let text = format!("{}{}\n", ok, bad);
        assert!(Index::from_text(&text).is_err(), "{}", bad);
    }
    assert!(Index::from_text("FORMING-CHESS-INDEX 1\n").is_err());
}

#[test]
fn one_bad_move_leaves_the_whole_game_out() {
    let mut ix = Index::new();
    let bad = game(1, GENERAL, START_FEN, &["e2e4", "e7e5", "e2e4"], GameResult::Draw);
    assert!(ix.merge(&bad).is_err());
    assert!(ix.nodes.is_empty() && ix.edges.is_empty() && !ix.is_merged(1));
}

fn append(path: &std::path::Path, text: &str) {
    use std::io::Write;
    std::fs::OpenOptions::new().append(true).open(path).unwrap().write_all(text.as_bytes()).unwrap();
}

#[test]
fn a_bad_game_is_skipped_and_later_games_still_merge() {
    let dir = temp_dir("badgame");
    let ar = Archive::new(&dir);
    let b = Board::start();
    // Game 1 lists a move that is not in its own legal-move listing.
    let w1 = ar.begin_game(GENERAL, &b, None).unwrap();
    append(&w1.path, &format!("M\t1\tw\te2e4\te4\t{}\t{}\td2d4,g1f3\n", "0".repeat(20), b.key()));
    append(&w1.path, "GAME END result=draw\n");
    let mut w2 = ar.begin_game(GENERAL, &b, None).unwrap();
    w2.add_move(1, &b, &b.parse_move("d2d4").unwrap(), &[0; 20]).unwrap();
    w2.finish(GameResult::Draw).unwrap();
    let mut ix = Index::new();
    let (n, errors) = ar.catch_up_report(&mut ix);
    assert_eq!(n, 1);
    assert_eq!(errors.len(), 1);
    assert!(!ix.is_merged(1) && ix.is_merged(2));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_cut_off_line_is_dropped_and_the_game_is_stopped() {
    let dir = temp_dir("cutoff");
    let ar = Archive::new(&dir);
    let b = Board::start();
    let mut w = ar.begin_game(GENERAL, &b, None).unwrap();
    w.add_move(1, &b, &b.parse_move("e2e4").unwrap(), &[0; 20]).unwrap();
    let path = w.path.clone();
    drop(w);
    let complete = std::fs::read_to_string(&path).unwrap();
    append(&path, "M\t2\tb\te7e5\te5\t000"); // killed in the middle of a line
    let ar = Archive::new(&dir);
    assert_eq!(ar.mark_unfinished_stopped().unwrap(), vec![1]);
    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(after, format!("{}GAME STOPPED\n", complete));
    // A later game still merges.
    let mut w2 = ar.begin_game(GENERAL, &b, None).unwrap();
    w2.add_move(1, &b, &b.parse_move("d2d4").unwrap(), &[0; 20]).unwrap();
    w2.finish(GameResult::Draw).unwrap();
    let mut ix = Index::new();
    let (n, errors) = ar.catch_up_report(&mut ix);
    assert_eq!((n, errors.len()), (2, 0));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unreadable_index_is_rebuilt_from_the_games() {
    let dir = temp_dir("badindex");
    let ar = Archive::new(&dir);
    let b = Board::start();
    let mut w = ar.begin_game(GENERAL, &b, None).unwrap();
    w.add_move(1, &b, &b.parse_move("e2e4").unwrap(), &[0; 20]).unwrap();
    w.finish(GameResult::Draw).unwrap();
    std::fs::write(ar.index_path(), "FORMING-CHESS-INDEX 2\nK\t0\tdrill\n").unwrap();
    let (ix, merged, notes) = ar.open_index();
    assert_eq!(merged, 1);
    assert!(ix.is_merged(1));
    assert_eq!(notes.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
