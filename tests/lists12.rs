//! Lists 1 and 2, the templates, the capture values and the history files.

use forming_chess::board::*;
use forming_chess::evaluate::{list1, list2};
use forming_chess::exchange::evaluate;
use forming_chess::history::{listing, History, HistoryRecord};
use forming_chess::strategies::TurnInfo;

fn ex(fen: &str, mv: &str) -> (i32, i32, i32) {
    let b = Board::from_fen(fen).unwrap();
    let m = b.parse_move(mv).unwrap_or_else(|| panic!("{} not legal in {}", mv, fen));
    let e = evaluate(&b, &m);
    (e.gain, e.loss, e.net())
}

#[test]
fn starting_list1_is_taking_the_center_and_king_shelter_at_30() {
    for white in [true, false] {
        let rows = list1(&Board::start(), white, &[]);
        let got: Vec<(&str, u32)> = rows.iter().map(|r| (r.name, r.score)).collect();
        assert_eq!(got, vec![("Central Pawn Occupation", 30), ("King Shelter", 30)]);
    }
}

#[test]
fn list1_never_uses_45_or_75() {
    let b = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    for white in [true, false] {
        for r in list1(&b, white, &[]) {
            assert!([30, 60, 100].contains(&r.score));
        }
    }
}

#[test]
fn central_pawn_break_completes_from_the_turn_window() {
    let b = Board::from_fen("4k3/8/8/3P4/8/8/8/4K3 w - - 0 1").unwrap();
    let window = vec![TurnInfo { white: true, from: 28, to: 35, piece: PAWN, captured: -PAWN }];
    let rows = list1(&b, true, &window);
    assert!(rows.iter().any(|r| r.name == "Central Pawn Break" && r.score == 100));
}

#[test]
fn promotion_template() {
    let b = Board::from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    let rows = list1(&b, true, &[]);
    assert!(rows.iter().any(|r| r.name == "Promotion" && r.score == 100));
    let blocked = Board::from_fen("n3k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    let rows = list1(&blocked, true, &[]);
    assert!(rows.iter().any(|r| r.name == "Promotion" && r.score == 60));
}

#[test]
fn capture_examples_from_step_14_3() {
    assert_eq!(ex("4k3/8/8/3p4/8/8/8/3QK3 w - - 0 1", "d1d5"), (100, 0, 100));
    assert_eq!(ex("4k3/2n5/8/3p4/8/8/8/3QK3 w - - 0 1", "d1d5"), (100, 900, -800));
    assert_eq!(ex("3rk3/8/8/3q4/4P3/8/8/3RK3 w - - 0 1", "e4d5"), (900, 0, 900));
    assert_eq!(ex("4k3/8/4p3/3q4/4P3/2N5/8/4K3 w - - 0 1", "c3d5"), (1000, 320, 680));
    assert_eq!(ex("4k3/8/2p5/3r4/8/8/8/3QK3 w - - 0 1", "d1d5"), (500, 900, -400));
    assert_eq!(ex("3rk3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6"), (100, 100, 0));
    assert_eq!(ex("7k/6pp/5P2/8/8/8/8/K5Q1 w - - 0 1", "g1g7"), (100_100, 0, 100_100));
    assert_eq!(ex("2k5/1P6/8/8/8/8/8/4K3 w - - 0 1", "b7b8q"), (800, 900, -100));
}

#[test]
fn list2_only_on_the_evaluated_players_turn() {
    let b = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    assert!(list2(&b, false, &[]).is_empty());
    let rows = list2(&b, true, &[]);
    assert!(!rows.is_empty());
    for w in rows.windows(2) {
        assert!(w[0].exchange.net() >= w[1].exchange.net());
    }
}

#[test]
fn history_files_keep_five_turns_with_full_listings() {
    let dir = std::env::temp_dir().join(format!("fc_hist_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut h = History::new(&dir);
    h.clear().unwrap();
    let mut b = Board::start();
    for (i, mv) in ["g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8"].iter().enumerate() {
        let m = b.parse_move(mv).unwrap();
        let a = b.make(&m);
        let white = b.white_to_move;
        h.push(HistoryRecord {
            ply: i as u32 + 1,
            move_no: b.fullmove,
            white,
            mv: m,
            san: b.san(&m),
            turn: TurnInfo { white, from: m.from, to: m.to, piece: b.sq[m.from as usize], captured: 0 },
            before: b,
            after: a,
        })
        .unwrap();
        b = a;
    }
    assert_eq!(h.window(true).len(), 5);
    let text = std::fs::read_to_string(h.path(true)).unwrap();
    assert_eq!(text.matches("RECORD").count(), 5);
    assert!(text.contains("castling rights: KQkq"));
    assert!(text.contains("en passant: -"));
    let l = listing(&Board::start());
    assert!(l.contains("pawn e2: e2e3 e2e4"));
    assert!(l.contains("pawn e7: e7e5 e7e6"));
}
