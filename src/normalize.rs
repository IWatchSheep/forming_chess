//! Step 15.3: position keys, with color and mirror folded in for the two endgames.

use crate::board::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cat {
    /// King and pawn against king.
    Kpk,
    /// King and rook against king.
    Krk,
    /// King and queen against king (only reached by promotion here).
    Kqk,
    /// King against king, or king and bishop or knight against king.
    DeadDraw,
    /// Any other position.
    Other,
}

impl Cat {
    pub fn code(&self) -> &'static str {
        match self {
            Cat::Kpk => "kpk",
            Cat::Krk => "krk",
            Cat::Kqk => "kqk",
            Cat::DeadDraw => "dead",
            Cat::Other => "other",
        }
    }
    pub fn from_code(s: &str) -> Cat {
        match s {
            "kpk" => Cat::Kpk,
            "krk" => Cat::Krk,
            "kqk" => Cat::Kqk,
            "dead" => Cat::DeadDraw,
            _ => Cat::Other,
        }
    }
    /// The two endgames where results are proven.
    pub fn proof_family(&self) -> bool {
        matches!(self, Cat::Kpk | Cat::Krk)
    }
    /// Endgames whose positions are flipped and mirrored.
    pub fn folded(&self) -> bool {
        !matches!(self, Cat::Other)
    }
}

pub fn categorize(b: &Board) -> Cat {
    // A position that still has castling rights is not folded.
    if b.castling != 0 {
        return Cat::Other;
    }
    let others: Vec<i8> = b.sq.iter().copied().filter(|&p| p != 0 && p.abs() != KING).collect();
    match others.len() {
        0 => Cat::DeadDraw,
        1 => match others[0].abs() {
            PAWN => Cat::Kpk,
            ROOK => Cat::Krk,
            QUEEN => Cat::Kqk,
            _ => Cat::DeadDraw,
        },
        _ => Cat::Other,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform {
    pub flip: bool,
    pub mirror: bool,
}

impl Transform {
    pub const IDENTITY: Transform = Transform { flip: false, mirror: false };

    pub fn sq(&self, s: u8) -> u8 {
        let mut f = file_of(s);
        let mut r = rank_of(s);
        if self.flip {
            r = 7 - r;
        }
        if self.mirror {
            f = 7 - f;
        }
        (r * 8 + f) as u8
    }

    /// The transform is its own inverse.
    pub fn mv(&self, m: &Move) -> Move {
        Move { from: self.sq(m.from), to: self.sq(m.to), promo: m.promo }
    }

    pub fn board(&self, b: &Board) -> Board {
        if !self.flip && !self.mirror {
            return *b;
        }
        let mut n = Board::empty();
        for s in 0..64u8 {
            let p = b.sq[s as usize];
            if p != 0 {
                n.sq[self.sq(s) as usize] = if self.flip { -p } else { p };
            }
        }
        n.white_to_move = if self.flip { !b.white_to_move } else { b.white_to_move };
        // Folded endgames have no castling rights and no en passant.
        n.castling = 0;
        n.ep = NO_SQ;
        n.halfmove = b.halfmove;
        n.fullmove = b.fullmove;
        n
    }

    pub fn move_text(&self, uci: &str) -> String {
        if !self.flip && !self.mirror {
            return uci.to_string();
        }
        let from = parse_sq(&uci[0..2]).unwrap_or(0);
        let to = parse_sq(&uci[2..4]).unwrap_or(0);
        let mut s = format!("{}{}", sq_name(self.sq(from)), sq_name(self.sq(to)));
        s.push_str(&uci[4..]);
        s
    }
}

#[derive(Clone, Debug)]
pub struct Normalized {
    pub board: Board,
    pub t: Transform,
    pub cat: Cat,
}

impl Normalized {
    pub fn key(&self) -> String {
        self.board.key()
    }
}

/// Fold color and mirror for the endgames; leave any other position as it is.
pub fn normalize(b: &Board) -> Normalized {
    let cat = categorize(b);
    if !cat.folded() {
        return Normalized { board: *b, t: Transform::IDENTITY, cat };
    }
    let piece_sq = (0..64u8).find(|&s| b.sq[s as usize] != 0 && b.sq[s as usize].abs() != KING);
    let flip = piece_sq.is_some_and(|s| b.sq[s as usize] < 0);
    let step1 = Transform { flip, mirror: false };
    let fb = step1.board(b);
    let mirror = match cat {
        Cat::Kpk => {
            let ps = (0..64u8).find(|&s| fb.sq[s as usize] == PAWN).unwrap_or(0);
            file_of(ps) >= 4
        }
        Cat::Krk | Cat::Kqk => file_of(fb.king_sq(true)) >= 4,
        _ => false,
    };
    let t = Transform { flip, mirror };
    Normalized { board: t.board(b), t, cat }
}

/// Is the position legal in the counted sense of Step 15.3: pieces on
/// different squares, kings not adjacent, the side not to move not in check.
pub fn counted_legal(b: &Board) -> bool {
    let wk = b.king_sq(true);
    let bk = b.king_sq(false);
    if wk == NO_SQ || bk == NO_SQ {
        return false;
    }
    if (file_of(wk) - file_of(bk)).abs() <= 1 && (rank_of(wk) - rank_of(bk)).abs() <= 1 {
        return false;
    }
    !b.in_check(!b.white_to_move)
}

/// Every counted king-and-pawn position with White's pawn on ranks 2 through 7.
/// With `folded`, only pawn files a through d.
pub fn kpk_positions(folded: bool) -> Vec<Board> {
    let mut out = Vec::new();
    for ps in 8..56u8 {
        if folded && file_of(ps) >= 4 {
            continue;
        }
        for wk in 0..64u8 {
            if wk == ps {
                continue;
            }
            for bk in 0..64u8 {
                if bk == ps || bk == wk {
                    continue;
                }
                for stm in [true, false] {
                    let mut b = Board::empty();
                    b.sq[ps as usize] = PAWN;
                    b.sq[wk as usize] = KING;
                    b.sq[bk as usize] = -KING;
                    b.white_to_move = stm;
                    if counted_legal(&b) {
                        out.push(b);
                    }
                }
            }
        }
    }
    out
}

/// The similar-position key for king and pawn against king (Step 12): signed
/// distances of both kings from the pawn, the pawn's rank, whether the pawn is
/// on a rook file, and the side to move. Read on the normalized board.
pub fn similar_key(n: &Normalized) -> Option<String> {
    if n.cat != Cat::Kpk {
        return None;
    }
    let b = &n.board;
    let ps = (0..64u8).find(|&s| b.sq[s as usize] == PAWN)?;
    let wk = b.king_sq(true);
    let bk = b.king_sq(false);
    let d = |k: u8| (file_of(k) - file_of(ps), rank_of(k) - rank_of(ps));
    let (wf, wr) = d(wk);
    let (bf, br) = d(bk);
    let rook_file = file_of(ps) == 0 || file_of(ps) == 7;
    Some(format!(
        "w{},{} b{},{} r{} {} {}",
        wf,
        wr,
        bf,
        br,
        rank_of(ps) + 1,
        if rook_file { "rookfile" } else { "inner" },
        if b.white_to_move { "w" } else { "b" }
    ))
}

/// A move described relative to its own start, for similar positions.
pub fn relative_move(b: &Board, m: &Move) -> String {
    let p = b.sq[m.from as usize];
    format!(
        "{}{:+}{:+}{}",
        piece_char(p),
        file_of(m.to) - file_of(m.from),
        rank_of(m.to) - rank_of(m.from),
        if m.promo != 0 { piece_char(-m.promo).to_string() } else { String::new() }
    )
}
