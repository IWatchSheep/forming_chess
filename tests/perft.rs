//! Move generation against published perft counts. Perft writes no file and fills no list.

use forming_chess::board::*;

#[test]
fn start_position_counts() {
    let b = Board::start();
    let expect = [20u64, 400, 8_902, 197_281];
    for (d, &n) in expect.iter().enumerate() {
        assert_eq!(perft(&b, d as u32 + 1), n, "depth {}", d + 1);
    }
}

#[test]
fn start_position_depth_5() {
    assert_eq!(perft(&Board::start(), 5), 4_865_609);
}

#[test]
fn kiwipete() {
    let b = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    assert_eq!(perft(&b, 1), 48);
    assert_eq!(perft(&b, 2), 2_039);
    assert_eq!(perft(&b, 3), 97_862);
}

#[test]
fn endgame_and_promotion_positions() {
    let b = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
    assert_eq!(perft(&b, 4), 43_238);
    let b = Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap();
    assert_eq!(perft(&b, 3), 9_467);
}
