//! The 20 strategy templates of Step 14.4.
//!
//! Each template reads the board (geometric attacks) and the evaluated
//! player's turn window, and reports whether its foundation, progress and
//! completion conditions are met. It also names the participating pieces and
//! the target squares, which Step 14.3 rule 8 uses to tie captures to a
//! strategy.

use crate::board::*;

pub const NAMES: [&str; 20] = [
    "Central Pawn Occupation",
    "Central Pawn Break",
    "Kingside Pawn Storm",
    "Queenside Minority Attack",
    "King Shelter",
    "Opposite-Side Castling Attack",
    "Fianchetto",
    "Open-File Control",
    "Seventh-Rank Invasion",
    "Passed-Pawn Advance",
    "Isolated-Pawn Attack",
    "Backward-Pawn Pressure",
    "Knight Outpost",
    "Bishop Pair",
    "Pin",
    "Discovered Attack",
    "Fork",
    "Skewer",
    "Major-Piece Battery",
    "Promotion",
];

pub const DESCRIPTIONS: [[&str; 3]; 20] = [
    ["A friendly pawn occupies the d-file or e-file.", "A friendly pawn occupies d4 or e4 for White, or d5 or e5 for Black.", "Both central squares are occupied by friendly pawns, and both pawns are defended."],
    ["The opponent has a central pawn, and a friendly pawn remains on a file from c through f.", "A friendly pawn attacks an opposing central pawn.", "The turn window contains a pawn capturing another pawn on the c-, d-, e-, or f-file."],
    ["At least two friendly pawns remain on the f-, g-, or h-file, at least one has moved, and the friendly king is outside those files.", "A participating pawn reaches relative rank 2.", "Two participating pawns reach relative rank 3 or higher, and the friendly king occupies a file from a through d."],
    ["The evaluated player has fewer pawns than the opponent on the a-, b-, and c-files, while retaining at least one pawn on those files.", "A friendly queenside pawn reaches relative rank 2.", "A participating pawn reaches relative rank 3, and an opposing pawn on the a- through d-files is isolated or doubled."],
    ["The king is on e1 or e8 with a remaining castling right, or is already on c1, g1, c8, or g8.", "The king has castled, or no more than one piece remains between the king and a castling rook.", "The king occupies a castled square, is not attacked, and its three shelter pawns remain unmoved."],
    ["The friendly king occupies the home rank on the a-c or f-h files, while the opposing king occupies the opposite wing.", "A friendly pawn on the opposing king's wing reaches relative rank 2.", "That pawn reaches relative rank 3, and a friendly rook or queen occupies a half-open file on that wing."],
    ["The b-pawn or g-pawn has advanced one square, or a friendly bishop already occupies b2, g2, b7, or g7.", "A friendly bishop occupies one of those fianchetto squares.", "The bishop attacks at least one of the central squares d4, d5, e4, or e5."],
    ["At least one file contains no friendly pawn, and a friendly rook or queen remains on the board.", "A friendly rook or queen occupies a file without a friendly pawn.", "A fully open file contains more friendly major pieces than opposing major pieces."],
    ["A friendly rook or queen exists, and at least one file contains no friendly pawn.", "A friendly major piece reaches relative rank 5.", "A friendly rook or queen reaches the opponent's second rank and attacks an opposing piece or pawn."],
    ["A friendly pawn has no opposing pawn ahead of it on its own file or either adjacent file.", "The passed pawn reaches relative rank 3.", "The passed pawn reaches relative rank 5 or higher and is defended."],
    ["An opposing pawn has no friendly neighboring pawn on an adjacent file.", "A friendly piece attacks that opposing pawn, or a friendly rook or queen shares its file.", "The number of attacks against the isolated pawn exceeds the number of defenses."],
    ["An opposing pawn's advance square is attacked by a friendly pawn, is not defended by an opposing pawn, and lies on a file without a friendly pawn.", "A friendly rook or queen occupies the opposing pawn's file.", "That major piece remains on the file, and the opposing pawn's advance square is occupied or attacked."],
    ["A square on relative ranks 3 through 6 is empty or contains a friendly knight, is defended by a friendly pawn, and cannot still be attacked by an opposing pawn.", "A friendly knight occupies or attacks that square.", "A friendly knight occupies the outpost square."],
    ["The evaluated player has two bishops, while the opponent has fewer than two.", "At least one friendly bishop has moved away from the back rank.", "At least one file from c through f contains no pawns, and a friendly bishop has three squares of unobstructed travel."],
    ["A friendly bishop, rook, or queen aligns with an opposing front piece and a more valuable rear piece or king.", "The attacking line to the opposing front piece is unobstructed.", "The rear piece is the opposing king, or a second friendly piece also attacks the front piece."],
    ["Exactly one friendly piece stands between a friendly sliding piece and an opposing piece other than a pawn.", "The intervening friendly piece has a legal move that takes it off the attacking line.", "That move captures a piece or delivers check, while the sliding piece continues attacking the original target."],
    ["A piece's current square or a possible legal move creates attacks against two opposing pieces other than pawns.", "Two opposing target pieces are currently attacked.", "Both targets satisfy the strategy's value conditions (kings, undefended, or at least as valuable as the attacker)."],
    ["A friendly sliding piece aligns with two opposing pieces, and the nearer opposing piece is at least as valuable as the farther piece.", "The line to the front opposing piece is unobstructed, and the front piece is the king or is strictly more valuable than the rear piece.", "The front piece is the king, or it is more valuable than the attacking sliding piece while the rear piece is undefended or worth at least as much as the attacker."],
    ["Two friendly rooks or queens share a rank or file, no more than two pieces stand between them, and at least one participating major piece has moved from its original square.", "No more than one piece remains between the participating major pieces.", "The line between the participating pieces is clear, and they share a half-open file or one attacks an opposing piece along that line."],
    ["A friendly pawn reaches relative rank 4 or higher.", "The pawn reaches relative rank 5 or higher.", "The pawn has an available legal promotion move. A pawn blocked on the seventh rank remains at the progress stage until a legal promotion move becomes available."],
];

/// One turn of the evaluated player's window (Step 5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnInfo {
    pub white: bool,
    pub from: u8,
    pub to: u8,
    pub piece: i8,
    pub captured: i8,
}

#[derive(Clone, Debug, Default)]
pub struct Reading {
    pub foundation: bool,
    pub progress: bool,
    pub completion: bool,
    pub participants: Vec<u8>,
    pub targets: Vec<u8>,
}

impl Reading {
    /// Three bits: foundation 1, progress 2, completion 4.
    pub fn flags(&self) -> u8 {
        (self.foundation as u8) | ((self.progress as u8) << 1) | ((self.completion as u8) << 2)
    }
}

pub const F_FOUNDATION: u8 = 1;
pub const F_PROGRESS: u8 = 2;
pub const F_COMPLETION: u8 = 4;

/// Score from Step 14.2 for List 1 (no look-ahead): 100, 60, 30, or 0 when not listed.
pub fn score_now(flags: u8) -> u32 {
    if flags & F_COMPLETION != 0 {
        100
    } else if flags & F_FOUNDATION != 0 && flags & F_PROGRESS != 0 {
        60
    } else if flags != 0 {
        30
    } else {
        0
    }
}

pub fn stage_name(score: u32) -> &'static str {
    match score {
        100 => "Fully developed",
        60 | 75 => "Partially developed",
        30 | 45 => "Beginning to form",
        _ => "-",
    }
}

pub fn stage_rank(score: u32) -> u8 {
    match score {
        100 => 0,
        60 | 75 => 1,
        30 | 45 => 2,
        _ => 3,
    }
}

/// Flags as a compact text: 20 digits, each 0..7.
pub fn flags_to_text(f: &[u8; 20]) -> String {
    f.iter().map(|d| (b'0' + d) as char).collect()
}

/// Sparse form for drill records: "-" or "index=flags" pairs, e.g. "9=3,19=1".
pub fn flags_to_sparse(f: &[u8; 20]) -> String {
    let parts: Vec<String> = f.iter().enumerate().filter(|(_, v)| **v != 0).map(|(i, v)| format!("{}={}", i, v)).collect();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(",")
    }
}

pub fn flags_from_sparse(s: &str) -> [u8; 20] {
    let mut f = [0u8; 20];
    for part in s.split(',') {
        if let Some((i, v)) = part.split_once('=') {
            if let (Ok(i), Ok(v)) = (i.parse::<usize>(), v.parse::<u8>()) {
                if i < 20 {
                    f[i] = v & 7;
                }
            }
        }
    }
    f
}

pub fn flags_from_text(s: &str) -> [u8; 20] {
    let mut f = [0u8; 20];
    for (i, c) in s.bytes().take(20).enumerate() {
        f[i] = c.saturating_sub(b'0') & 7;
    }
    f
}

pub struct Ctx<'a> {
    pub b: &'a Board,
    pub white: bool,
    pub window: &'a [TurnInfo],
}

fn friendly(p: i8, white: bool) -> bool {
    if white {
        p > 0
    } else {
        p < 0
    }
}
fn enemy(p: i8, white: bool) -> bool {
    p != 0 && !friendly(p, white)
}
fn rel_rank(sq: u8, white: bool) -> i8 {
    if white {
        rank_of(sq)
    } else {
        7 - rank_of(sq)
    }
}
/// Square from a relative file and rank for the evaluated player.
fn rel_sq(file: i8, rrank: i8, white: bool) -> u8 {
    let r = if white { rrank } else { 7 - rrank };
    (r * 8 + file) as u8
}
fn own(white: bool, t: i8) -> i8 {
    if white {
        t
    } else {
        -t
    }
}
fn squares_of(b: &Board, p: i8) -> Vec<u8> {
    (0..64u8).filter(|&s| b.sq[s as usize] == p).collect()
}
fn file_has(b: &Board, file: i8, p: i8) -> bool {
    (0..8).any(|r| b.sq[(r * 8 + file) as usize] == p)
}
fn compare_value(p: i8) -> i32 {
    if p.abs() == KING {
        10_000
    } else {
        piece_value(p)
    }
}
fn is_major(p: i8) -> bool {
    p.abs() == ROOK || p.abs() == QUEEN
}
fn is_slider(p: i8) -> bool {
    matches!(p.abs(), BISHOP | ROOK | QUEEN)
}
fn slider_dirs(p: i8) -> Vec<(i8, i8)> {
    let mut d = Vec::new();
    if p.abs() == BISHOP || p.abs() == QUEEN {
        d.extend_from_slice(&[(1, 1), (1, -1), (-1, 1), (-1, -1)]);
    }
    if p.abs() == ROOK || p.abs() == QUEEN {
        d.extend_from_slice(&[(1, 0), (-1, 0), (0, 1), (0, -1)]);
    }
    d
}
fn ray(from: u8, d: (i8, i8)) -> Vec<u8> {
    let mut out = Vec::new();
    let mut f = file_of(from) + d.0;
    let mut r = rank_of(from) + d.1;
    while let Some(s) = make_sq(f, r) {
        out.push(s);
        f += d.0;
        r += d.1;
    }
    out
}
fn defended(b: &Board, sq: u8, by_white: bool) -> bool {
    b.attacked_by(sq, by_white)
}

/// Read every template for one side.
pub fn read_all(ctx: &Ctx) -> Vec<Reading> {
    (0..20).map(|i| read(i, ctx)).collect()
}

pub fn flags_all(ctx: &Ctx) -> [u8; 20] {
    let mut f = [0u8; 20];
    for (i, r) in read_all(ctx).iter().enumerate() {
        f[i] = r.flags();
    }
    f
}

pub fn read(id: usize, ctx: &Ctx) -> Reading {
    match id {
        0 => central_occupation(ctx),
        1 => central_break(ctx),
        2 => kingside_storm(ctx),
        3 => minority_attack(ctx),
        4 => king_shelter(ctx),
        5 => opposite_castling(ctx),
        6 => fianchetto(ctx),
        7 => open_file(ctx),
        8 => seventh_rank(ctx),
        9 => passed_pawn(ctx),
        10 => isolated_attack(ctx),
        11 => backward_pressure(ctx),
        12 => knight_outpost(ctx),
        13 => bishop_pair(ctx),
        14 => pin(ctx),
        15 => discovered(ctx),
        16 => fork(ctx),
        17 => skewer(ctx),
        18 => battery(ctx),
        _ => promotion(ctx),
    }
}

fn central_squares(white: bool) -> [u8; 2] {
    if white {
        [27, 28] // d4, e4
    } else {
        [35, 36] // d5, e5
    }
}

fn central_occupation(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let pawn = own(w, PAWN);
    let mut r = Reading::default();
    let pawns: Vec<u8> = squares_of(b, pawn).into_iter().filter(|&s| file_of(s) == 3 || file_of(s) == 4).collect();
    r.foundation = !pawns.is_empty();
    let cs = central_squares(w);
    r.progress = cs.iter().any(|&s| b.sq[s as usize] == pawn);
    r.completion = cs.iter().all(|&s| b.sq[s as usize] == pawn && defended(b, s, w));
    r.participants = pawns;
    r.targets = vec![27, 28, 35, 36];
    r
}

/// Opposing pawns on d4, e4, d5 or e5.
fn opposing_central_pawns(b: &Board, w: bool) -> Vec<u8> {
    [27u8, 28, 35, 36].into_iter().filter(|&s| b.sq[s as usize] == own(!w, PAWN)).collect()
}

fn central_break(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let opp = opposing_central_pawns(b, w);
    let mine: Vec<u8> = squares_of(b, own(w, PAWN)).into_iter().filter(|&s| (2..=5).contains(&file_of(s))).collect();
    r.foundation = !opp.is_empty() && !mine.is_empty();
    r.progress = opp.iter().any(|&t| mine.iter().any(|&m| b.attacks_from(m).contains(&t)));
    r.completion = c.window.iter().any(|t| {
        t.piece.abs() == PAWN && t.captured.abs() == PAWN && (2..=5).contains(&file_of(t.to))
    });
    r.participants = mine;
    r.targets = opp;
    r
}

fn kingside_storm(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let pawns: Vec<u8> = squares_of(b, own(w, PAWN)).into_iter().filter(|&s| file_of(s) >= 5).collect();
    let k = b.king_sq(w);
    let king_outside = k != NO_SQ && file_of(k) < 5;
    let moved = pawns.iter().any(|&s| rel_rank(s, w) >= 2);
    r.foundation = pawns.len() >= 2 && moved && king_outside;
    r.progress = pawns.iter().any(|&s| rel_rank(s, w) >= 2);
    r.completion = pawns.iter().filter(|&&s| rel_rank(s, w) >= 3).count() >= 2 && k != NO_SQ && file_of(k) <= 3;
    r.participants = pawns;
    r.targets = (0..64u8).filter(|&s| file_of(s) >= 5).collect();
    r
}

fn pawns_on_files(b: &Board, p: i8, files: std::ops::RangeInclusive<i8>) -> Vec<u8> {
    squares_of(b, p).into_iter().filter(|&s| files.contains(&file_of(s))).collect()
}

fn pawn_isolated(b: &Board, sq: u8) -> bool {
    let p = b.sq[sq as usize];
    let f = file_of(sq);
    !(f > 0 && file_has(b, f - 1, p)) && !(f < 7 && file_has(b, f + 1, p))
}

fn pawn_doubled(b: &Board, sq: u8) -> bool {
    let p = b.sq[sq as usize];
    (0..8).filter(|&r| b.sq[(r * 8 + file_of(sq)) as usize] == p).count() >= 2
}

fn minority_attack(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let mine = pawns_on_files(b, own(w, PAWN), 0..=2);
    let theirs = pawns_on_files(b, own(!w, PAWN), 0..=2);
    r.foundation = !mine.is_empty() && mine.len() < theirs.len();
    r.progress = mine.iter().any(|&s| rel_rank(s, w) >= 2);
    let weak = pawns_on_files(b, own(!w, PAWN), 0..=3).into_iter().any(|s| pawn_isolated(b, s) || pawn_doubled(b, s));
    r.completion = mine.iter().any(|&s| rel_rank(s, w) >= 3) && weak;
    r.participants = mine;
    r.targets = (0..64u8).filter(|&s| file_of(s) <= 3 && enemy(b.sq[s as usize], w)).collect();
    r
}

fn king_shelter(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let k = b.king_sq(w);
    if k == NO_SQ {
        return r;
    }
    let (kbit, qbit) = if w { (CASTLE_WK, CASTLE_WQ) } else { (CASTLE_BK, CASTLE_BQ) };
    let home = rel_sq(4, 0, w);
    let castled_sqs = [rel_sq(2, 0, w), rel_sq(6, 0, w)];
    let has_right = b.castling & (kbit | qbit) != 0;
    r.foundation = (k == home && has_right) || castled_sqs.contains(&k);
    let between = |a: u8, z: u8| -> usize {
        let (lo, hi) = if a < z { (a, z) } else { (z, a) };
        ((lo + 1)..hi).filter(|&s| b.sq[s as usize] != 0).count()
    };
    let mut near_rook = false;
    if k == home {
        if b.castling & kbit != 0 && between(k, rel_sq(7, 0, w)) <= 1 {
            near_rook = true;
        }
        if b.castling & qbit != 0 && between(k, rel_sq(0, 0, w)) <= 1 {
            near_rook = true;
        }
    }
    r.progress = castled_sqs.contains(&k) || near_rook;
    if castled_sqs.contains(&k) {
        let kf = file_of(k);
        let files: [i8; 3] = if kf >= 4 { [5, 6, 7] } else { [1, 2, 3] };
        let shelter_unmoved = files.iter().all(|&f| b.sq[rel_sq(f, 1, w) as usize] == own(w, PAWN));
        r.completion = !b.attacked_by(k, !w) && shelter_unmoved;
        r.participants = files.iter().map(|&f| rel_sq(f, 1, w)).collect();
    }
    r.participants.push(k);
    r.targets = b.attacks_from(k);
    r
}

fn opposite_castling(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let k = b.king_sq(w);
    let ok = b.king_sq(!w);
    if k == NO_SQ || ok == NO_SQ {
        return r;
    }
    let my_wing_q = file_of(k) <= 2;
    let my_wing_k = file_of(k) >= 5;
    let opp_files: std::ops::RangeInclusive<i8> = if my_wing_q { 5..=7 } else { 0..=2 };
    r.foundation =
        rel_rank(k, w) == 0 && (my_wing_q || my_wing_k) && opp_files.contains(&file_of(ok));
    let wing_pawns: Vec<u8> = squares_of(b, own(w, PAWN)).into_iter().filter(|&s| opp_files.contains(&file_of(ok)) && opp_files.contains(&file_of(s))).collect();
    r.progress = wing_pawns.iter().any(|&s| rel_rank(s, w) >= 2);
    let half_open_major = (0..64u8).any(|s| {
        let p = b.sq[s as usize];
        friendly(p, w)
            && is_major(p)
            && opp_files.contains(&file_of(s))
            && !file_has(b, file_of(s), own(w, PAWN))
            && file_has(b, file_of(s), own(!w, PAWN))
    });
    r.completion = r.foundation && wing_pawns.iter().any(|&s| rel_rank(s, w) >= 3) && half_open_major;
    r.participants = (0..64u8)
        .filter(|&s| {
            let p = b.sq[s as usize];
            friendly(p, w) && (p.abs() == PAWN || is_major(p)) && opp_files.contains(&file_of(s))
        })
        .collect();
    r.targets = (0..64u8).filter(|&s| opp_files.contains(&file_of(s))).collect();
    r
}

fn fianchetto(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let fsq = [rel_sq(1, 1, w), rel_sq(6, 1, w)];
    let pawn_adv = [rel_sq(1, 2, w), rel_sq(6, 2, w)].iter().any(|&s| b.sq[s as usize] == own(w, PAWN));
    let bishops: Vec<u8> = fsq.iter().copied().filter(|&s| b.sq[s as usize] == own(w, BISHOP)).collect();
    r.foundation = pawn_adv || !bishops.is_empty();
    r.progress = !bishops.is_empty();
    r.completion = bishops.iter().any(|&s| b.attacks_from(s).iter().any(|t| [27u8, 28, 35, 36].contains(t)));
    r.participants = bishops;
    r.targets = vec![27, 28, 35, 36];
    r
}

fn open_file(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let pawn = own(w, PAWN);
    let no_pawn_files: Vec<i8> = (0..8).filter(|&f| !file_has(b, f, pawn)).collect();
    let majors: Vec<u8> = (0..64u8).filter(|&s| friendly(b.sq[s as usize], w) && is_major(b.sq[s as usize])).collect();
    r.foundation = !no_pawn_files.is_empty() && !majors.is_empty();
    r.progress = majors.iter().any(|&s| no_pawn_files.contains(&file_of(s)));
    r.completion = (0..8).any(|f| {
        if file_has(b, f, PAWN) || file_has(b, f, -PAWN) {
            return false;
        }
        let mine = (0..8).filter(|&rk| {
            let p = b.sq[(rk * 8 + f) as usize];
            friendly(p, w) && is_major(p)
        }).count();
        let theirs = (0..8).filter(|&rk| {
            let p = b.sq[(rk * 8 + f) as usize];
            enemy(p, w) && is_major(p)
        }).count();
        mine > theirs
    });
    r.participants = majors;
    r.targets = (0..64u8).filter(|&s| no_pawn_files.contains(&file_of(s))).collect();
    r
}

fn seventh_rank(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let majors: Vec<u8> = (0..64u8).filter(|&s| friendly(b.sq[s as usize], w) && is_major(b.sq[s as usize])).collect();
    r.foundation = !majors.is_empty() && (0..8).any(|f| !file_has(b, f, own(w, PAWN)));
    r.progress = majors.iter().any(|&s| rel_rank(s, w) >= 5);
    r.completion = majors.iter().any(|&s| {
        rel_rank(s, w) == 6 && b.attacks_from(s).iter().any(|&t| enemy(b.sq[t as usize], w) && b.sq[t as usize].abs() != KING)
    });
    r.participants = majors;
    r.targets = (0..8).map(|f| rel_sq(f, 6, w)).collect();
    r
}

fn is_passed(b: &Board, sq: u8, w: bool) -> bool {
    let f = file_of(sq);
    let rr = rel_rank(sq, w);
    for df in -1..=1 {
        let ff = f + df;
        if !(0..8).contains(&ff) {
            continue;
        }
        for r2 in (rr + 1)..8 {
            if b.sq[rel_sq(ff, r2, w) as usize] == own(!w, PAWN) {
                return false;
            }
        }
    }
    true
}

fn passed_pawn(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let passed: Vec<u8> = squares_of(b, own(w, PAWN)).into_iter().filter(|&s| is_passed(b, s, w)).collect();
    r.foundation = !passed.is_empty();
    r.progress = passed.iter().any(|&s| rel_rank(s, w) >= 3);
    r.completion = passed.iter().any(|&s| rel_rank(s, w) >= 5 && defended(b, s, w));
    for &s in &passed {
        let f = file_of(s);
        for df in -1..=1 {
            let ff = f + df;
            if (0..8).contains(&ff) {
                for r2 in (rel_rank(s, w) + 1)..8 {
                    r.targets.push(rel_sq(ff, r2, w));
                }
            }
        }
    }
    r.participants = passed;
    r
}

fn isolated_attack(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let iso: Vec<u8> = squares_of(b, own(!w, PAWN)).into_iter().filter(|&s| pawn_isolated(b, s)).collect();
    r.foundation = !iso.is_empty();
    let mut parts = Vec::new();
    for &s in &iso {
        let att: Vec<u8> = b.attackers(s, w).collect();
        let file_major = (0..64u8).any(|q| friendly(b.sq[q as usize], w) && is_major(b.sq[q as usize]) && file_of(q) == file_of(s));
        if !att.is_empty() || file_major {
            r.progress = true;
        }
        let def = b.attackers(s, !w).count();
        if att.len() > def {
            r.completion = true;
        }
        parts.extend(att);
    }
    parts.sort_unstable();
    parts.dedup();
    r.participants = parts;
    r.targets = iso;
    r
}

fn backward_pressure(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let mut targets = Vec::new();
    for s in squares_of(b, own(!w, PAWN)) {
        // The opposing pawn advances toward the evaluated player's side.
        let adv_rank = rank_of(s) + if w { -1 } else { 1 };
        let Some(adv) = make_sq(file_of(s), adv_rank) else { continue };
        let attacked_by_my_pawn = b.attackers(adv, w).any(|a| b.sq[a as usize] == own(w, PAWN));
        let defended_by_their_pawn = b.attackers(adv, !w).any(|a| b.sq[a as usize] == own(!w, PAWN));
        if attacked_by_my_pawn && !defended_by_their_pawn && !file_has(b, file_of(s), own(w, PAWN)) {
            r.foundation = true;
            let major_on_file: Vec<u8> = (0..8)
                .map(|rk| (rk * 8 + file_of(s)) as u8)
                .filter(|&q| friendly(b.sq[q as usize], w) && is_major(b.sq[q as usize]))
                .collect();
            if !major_on_file.is_empty() {
                r.progress = true;
                if b.sq[adv as usize] != 0 && friendly(b.sq[adv as usize], w) || b.attacked_by(adv, w) {
                    r.completion = true;
                }
            }
            r.participants.extend(major_on_file);
            targets.push(s);
            targets.push(adv);
        }
    }
    r.targets = targets;
    r
}

fn outpost_squares(b: &Board, w: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for rr in 3..=6 {
        for f in 0..8 {
            let s = rel_sq(f, rr, w);
            let p = b.sq[s as usize];
            if !(p == 0 || p == own(w, KNIGHT)) {
                continue;
            }
            let by_pawn = b.attackers(s, w).any(|a| b.sq[a as usize] == own(w, PAWN));
            if !by_pawn {
                continue;
            }
            // An opposing pawn on an adjacent file that has not yet passed this square could still attack it.
            let threat = [f - 1, f + 1].iter().filter(|&&ff| (0..8).contains(&ff)).any(|&ff| {
                ((rr + 1)..8).any(|r2| b.sq[rel_sq(ff, r2, w) as usize] == own(!w, PAWN))
            });
            if !threat {
                out.push(s);
            }
        }
    }
    out
}

fn knight_outpost(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let posts = outpost_squares(b, w);
    let knights = squares_of(b, own(w, KNIGHT));
    r.foundation = !posts.is_empty();
    r.progress = posts.iter().any(|&p| knights.contains(&p) || knights.iter().any(|&k| b.attacks_from(k).contains(&p)));
    r.completion = posts.iter().any(|&p| knights.contains(&p));
    r.participants = knights;
    r.targets = posts;
    r
}

fn bishop_pair(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let mine = squares_of(b, own(w, BISHOP));
    let theirs = squares_of(b, own(!w, BISHOP));
    r.foundation = mine.len() >= 2 && theirs.len() < 2;
    r.progress = mine.iter().any(|&s| rel_rank(s, w) != 0);
    let open_center = (2..=5).any(|f| !file_has(b, f, PAWN) && !file_has(b, f, -PAWN));
    let travel = mine.iter().any(|&s| b.attacks_from(s).iter().filter(|&&t| b.sq[t as usize] == 0).count() >= 3);
    r.completion = r.foundation && open_center && travel;
    r.targets = mine.iter().flat_map(|&s| b.attacks_from(s)).filter(|&t| enemy(b.sq[t as usize], w)).collect();
    r.participants = mine;
    r
}

/// Pieces along a ray from a friendly slider, for Pin and Skewer:
/// an optional opposing blocker, then a front and a rear opposing piece with
/// nothing between them. A friendly piece on the ray ends it.
struct LineHit {
    slider: u8,
    blocker: Option<u8>,
    front: u8,
    rear: u8,
}

fn line_hits(b: &Board, w: bool) -> Vec<LineHit> {
    let mut out = Vec::new();
    for s in 0..64u8 {
        let p = b.sq[s as usize];
        if !(friendly(p, w) && is_slider(p)) {
            continue;
        }
        for d in slider_dirs(p) {
            let pieces: Vec<u8> = ray(s, d).into_iter().filter(|&q| b.sq[q as usize] != 0).take(3).collect();
            if pieces.iter().any(|&q| friendly(b.sq[q as usize], w)) {
                // Stop at the first friendly piece.
                let cut: Vec<u8> = pieces.iter().copied().take_while(|&q| !friendly(b.sq[q as usize], w)).collect();
                push_hit(&cut, s, &mut out);
            } else {
                push_hit(&pieces, s, &mut out);
            }
        }
    }
    out
}

fn push_hit(pieces: &[u8], s: u8, out: &mut Vec<LineHit>) {
    if pieces.len() >= 2 {
        out.push(LineHit { slider: s, blocker: None, front: pieces[0], rear: pieces[1] });
    }
    if pieces.len() >= 3 {
        out.push(LineHit { slider: s, blocker: Some(pieces[0]), front: pieces[1], rear: pieces[2] });
    }
}

fn pin(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    for h in line_hits(b, w) {
        let front = b.sq[h.front as usize];
        let rear = b.sq[h.rear as usize];
        if front.abs() == KING {
            continue;
        }
        if !(rear.abs() == KING || piece_value(rear) > piece_value(front)) {
            continue;
        }
        r.foundation = true;
        r.participants.push(h.slider);
        r.targets.push(h.front);
        r.targets.push(h.rear);
        if h.blocker.is_none() {
            r.progress = true;
            let second = b.attackers(h.front, w).any(|a| a != h.slider);
            if rear.abs() == KING || second {
                r.completion = true;
            }
        }
    }
    r
}

fn discovered(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let legal = b.legal_moves_for(w);
    for s in 0..64u8 {
        let p = b.sq[s as usize];
        if !(friendly(p, w) && is_slider(p)) {
            continue;
        }
        for d in slider_dirs(p) {
            let line = ray(s, d);
            let pieces: Vec<u8> = line.iter().copied().filter(|&q| b.sq[q as usize] != 0).take(2).collect();
            if pieces.len() < 2 {
                continue;
            }
            let (mid, tgt) = (pieces[0], pieces[1]);
            if !friendly(b.sq[mid as usize], w) || !enemy(b.sq[tgt as usize], w) || b.sq[tgt as usize].abs() == PAWN {
                continue;
            }
            r.foundation = true;
            r.participants.push(s);
            r.participants.push(mid);
            r.targets.push(tgt);
            for m in legal.iter().filter(|m| m.from == mid && !line.contains(&m.to)) {
                r.progress = true;
                let after = b.make(m);
                let captures = b.is_capture(m);
                let check = after.in_check(!w);
                let still = after.attacks_from(s).contains(&tgt);
                if (captures || check) && still {
                    r.completion = true;
                    let cs = b.captured_sq(m);
                    if captures {
                        r.targets.push(cs);
                    }
                }
            }
        }
    }
    r
}

fn fork_targets_ok(b: &Board, attacker: i8, targets: &[u8], w: bool) -> bool {
    targets.iter().all(|&t| {
        let v = b.sq[t as usize];
        v.abs() == KING || !b.attacked_by(t, !w) || compare_value(v) >= compare_value(attacker)
    })
}

fn forked(b: &Board, from: u8, w: bool) -> Vec<u8> {
    b.attacks_from(from)
        .into_iter()
        .filter(|&t| enemy(b.sq[t as usize], w) && b.sq[t as usize].abs() != PAWN)
        .collect()
}

fn fork(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    for s in 0..64u8 {
        let p = b.sq[s as usize];
        if !friendly(p, w) {
            continue;
        }
        let t = forked(b, s, w);
        if t.len() >= 2 {
            r.foundation = true;
            r.progress = true;
            r.participants.push(s);
            r.targets.extend(t.iter().copied());
            // Completion needs two targets that both qualify.
            let good: Vec<u8> = t.iter().copied().filter(|&x| fork_targets_ok(b, p, &[x], w)).collect();
            if good.len() >= 2 {
                r.completion = true;
            }
        }
    }
    for m in b.legal_moves_for(w) {
        let after = b.make(&m);
        let t = forked(&after, m.to, w);
        if t.len() >= 2 {
            r.foundation = true;
            r.participants.push(m.from);
            r.targets.extend(t.iter().copied());
        }
    }
    r.participants.sort_unstable();
    r.participants.dedup();
    r.targets.sort_unstable();
    r.targets.dedup();
    r
}

fn skewer(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    for h in line_hits(b, w) {
        let front = b.sq[h.front as usize];
        let rear = b.sq[h.rear as usize];
        if compare_value(front) < compare_value(rear) {
            continue;
        }
        r.foundation = true;
        r.participants.push(h.slider);
        r.targets.push(h.front);
        r.targets.push(h.rear);
        if h.blocker.is_none() && (front.abs() == KING || compare_value(front) > compare_value(rear)) {
            r.progress = true;
            let sv = piece_value(b.sq[h.slider as usize]);
            let rear_ok = !b.attacked_by(h.rear, !w) || piece_value(rear) >= sv;
            if front.abs() == KING || (piece_value(front) > sv && rear_ok) {
                r.completion = true;
            }
        }
    }
    r
}

fn battery(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let majors: Vec<u8> = (0..64u8).filter(|&s| friendly(b.sq[s as usize], w) && is_major(b.sq[s as usize])).collect();
    let original = |s: u8| -> bool {
        let p = b.sq[s as usize];
        (p.abs() == ROOK && (s == rel_sq(0, 0, w) || s == rel_sq(7, 0, w))) || (p.abs() == QUEEN && s == rel_sq(3, 0, w))
    };
    for i in 0..majors.len() {
        for j in (i + 1)..majors.len() {
            let (a, z) = (majors[i], majors[j]);
            let same_file = file_of(a) == file_of(z);
            let same_rank = rank_of(a) == rank_of(z);
            if !(same_file || same_rank) {
                continue;
            }
            let step: i8 = if same_file { 8 } else { 1 };
            let between: Vec<u8> = {
                let (lo, hi) = if a < z { (a as i8, z as i8) } else { (z as i8, a as i8) };
                let mut v = Vec::new();
                let mut s = lo + step;
                while s < hi {
                    if b.sq[s as usize] != 0 {
                        v.push(s as u8);
                    }
                    s += step;
                }
                v
            };
            if between.len() > 2 || (original(a) && original(z)) {
                continue;
            }
            r.foundation = true;
            r.participants.push(a);
            r.participants.push(z);
            if between.len() <= 1 {
                r.progress = true;
            }
            if between.is_empty() {
                let half_open = same_file && !file_has(b, file_of(a), own(w, PAWN));
                let attacks_along = [a, z].iter().any(|&s| {
                    let d = if same_file {
                        if s == a.min(z) { (0, -1) } else { (0, 1) }
                    } else if s == a.min(z) {
                        (-1, 0)
                    } else {
                        (1, 0)
                    };
                    ray(s, d).into_iter().find(|&q| b.sq[q as usize] != 0).is_some_and(|q| enemy(b.sq[q as usize], w))
                });
                if half_open || attacks_along {
                    r.completion = true;
                }
                // Targets: squares along the shared line.
                for d in if same_file { [(0, 1), (0, -1)] } else { [(1, 0), (-1, 0)] } {
                    r.targets.extend(ray(a, d));
                }
            }
        }
    }
    r
}

fn promotion(c: &Ctx) -> Reading {
    let b = c.b;
    let w = c.white;
    let mut r = Reading::default();
    let pawns: Vec<u8> = squares_of(b, own(w, PAWN)).into_iter().filter(|&s| rel_rank(s, w) >= 4).collect();
    r.foundation = !pawns.is_empty();
    r.progress = pawns.iter().any(|&s| rel_rank(s, w) >= 5);
    r.completion = b.legal_moves_for(w).iter().any(|m| m.promo != 0);
    for &s in &pawns {
        r.targets.extend(b.attacks_from(s));
        if let Some(t) = make_sq(file_of(s), rank_of(s) + if w { 1 } else { -1 }) {
            r.targets.push(t);
        }
    }
    r.participants = pawns;
    r
}
