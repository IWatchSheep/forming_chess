//! Board, legal move generation, SAN, check, mate and draws.
//!
//! Squares are numbered a1 = 0, b1 = 1, ..., h1 = 7, a2 = 8, ..., h8 = 63.
//! Pieces are signed: 1 pawn, 2 knight, 3 bishop, 4 rook, 5 queen, 6 king.
//! White pieces are positive, Black pieces negative, 0 is an empty square.

use std::fmt::Write as _;

pub const PAWN: i8 = 1;
pub const KNIGHT: i8 = 2;
pub const BISHOP: i8 = 3;
pub const ROOK: i8 = 4;
pub const QUEEN: i8 = 5;
pub const KING: i8 = 6;

pub const CASTLE_WK: u8 = 1;
pub const CASTLE_WQ: u8 = 2;
pub const CASTLE_BK: u8 = 4;
pub const CASTLE_BQ: u8 = 8;

pub const NO_SQ: u8 = 255;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

const KNIGHT_D: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
const KING_D: [(i8, i8); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
const BISHOP_D: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const ROOK_D: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

#[inline]
pub fn file_of(sq: u8) -> i8 {
    (sq % 8) as i8
}
#[inline]
pub fn rank_of(sq: u8) -> i8 {
    (sq / 8) as i8
}
#[inline]
pub fn make_sq(file: i8, rank: i8) -> Option<u8> {
    if (0..8).contains(&file) && (0..8).contains(&rank) {
        Some((rank * 8 + file) as u8)
    } else {
        None
    }
}

pub fn sq_name(sq: u8) -> String {
    let f = (b'a' + (sq % 8)) as char;
    let r = (b'1' + (sq / 8)) as char;
    format!("{}{}", f, r)
}

pub fn parse_sq(s: &str) -> Option<u8> {
    let b = s.as_bytes();
    if b.len() != 2 {
        return None;
    }
    let f = b[0].to_ascii_lowercase();
    let r = b[1];
    if !(b'a'..=b'h').contains(&f) || !(b'1'..=b'8').contains(&r) {
        return None;
    }
    Some((r - b'1') * 8 + (f - b'a'))
}

pub fn piece_char(p: i8) -> char {
    let c = match p.abs() {
        PAWN => 'p',
        KNIGHT => 'n',
        BISHOP => 'b',
        ROOK => 'r',
        QUEEN => 'q',
        KING => 'k',
        _ => '.',
    };
    if p > 0 {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

pub fn piece_name(p: i8) -> &'static str {
    match p.abs() {
        PAWN => "pawn",
        KNIGHT => "knight",
        BISHOP => "bishop",
        ROOK => "rook",
        QUEEN => "queen",
        KING => "king",
        _ => "none",
    }
}

/// Values from Step 14.3. The king has no capture value.
pub fn piece_value(p: i8) -> i32 {
    match p.abs() {
        PAWN => 100,
        KNIGHT => 320,
        BISHOP => 330,
        ROOK => 500,
        QUEEN => 900,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    /// Promotion piece type (2..=5), or 0.
    pub promo: i8,
}

impl Move {
    pub fn uci(&self) -> String {
        let mut s = format!("{}{}", sq_name(self.from), sq_name(self.to));
        if self.promo != 0 {
            s.push(piece_char(-self.promo));
        }
        s
    }
}

/// Promotion order used by the Step 4 listing: queen, rook, bishop, knight.
pub fn promo_rank(p: i8) -> u8 {
    match p {
        QUEEN => 0,
        ROOK => 1,
        BISHOP => 2,
        KNIGHT => 3,
        _ => 4,
    }
}

/// Step 4 listing order: origin a1..h8, then destination a1..h8, then queen, rook, bishop, knight.
pub fn listing_key(m: &Move) -> (u8, u8, u8) {
    (m.from, m.to, promo_rank(m.promo))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Board {
    pub sq: [i8; 64],
    pub white_to_move: bool,
    pub castling: u8,
    pub ep: u8,
    pub halfmove: u32,
    pub fullmove: u32,
}

impl Default for Board {
    fn default() -> Self {
        Board::start()
    }
}

impl Board {
    pub fn empty() -> Board {
        Board { sq: [0; 64], white_to_move: true, castling: 0, ep: NO_SQ, halfmove: 0, fullmove: 1 }
    }

    pub fn start() -> Board {
        Board::from_fen(START_FEN).expect("start position")
    }

    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("a FEN needs at least the placement and the side to move".into());
        }
        let mut b = Board::empty();
        let rows: Vec<&str> = parts[0].split('/').collect();
        if rows.len() != 8 {
            return Err("the placement needs 8 ranks".into());
        }
        for (i, row) in rows.iter().enumerate() {
            let rank = 7 - i as i8;
            let mut file: i8 = 0;
            for c in row.chars() {
                if let Some(d) = c.to_digit(10) {
                    file += d as i8;
                    continue;
                }
                let t = match c.to_ascii_lowercase() {
                    'p' => PAWN,
                    'n' => KNIGHT,
                    'b' => BISHOP,
                    'r' => ROOK,
                    'q' => QUEEN,
                    'k' => KING,
                    _ => return Err(format!("unknown piece '{}'", c)),
                };
                let s = make_sq(file, rank).ok_or("a rank has too many squares")?;
                b.sq[s as usize] = if c.is_ascii_uppercase() { t } else { -t };
                file += 1;
            }
            if file != 8 {
                return Err("a rank does not have 8 squares".into());
            }
        }
        b.white_to_move = match parts[1] {
            "w" => true,
            "b" => false,
            _ => return Err("side to move must be w or b".into()),
        };
        if parts.len() > 2 {
            for c in parts[2].chars() {
                b.castling |= match c {
                    'K' => CASTLE_WK,
                    'Q' => CASTLE_WQ,
                    'k' => CASTLE_BK,
                    'q' => CASTLE_BQ,
                    _ => 0,
                };
            }
        }
        if parts.len() > 3 && parts[3] != "-" {
            b.ep = parse_sq(parts[3]).ok_or("bad en passant square")?;
        }
        if parts.len() > 4 {
            b.halfmove = parts[4].parse().unwrap_or(0);
        }
        if parts.len() > 5 {
            b.fullmove = parts[5].parse().unwrap_or(1);
        }
        let wk = b.sq.iter().filter(|&&p| p == KING).count();
        let bk = b.sq.iter().filter(|&&p| p == -KING).count();
        if wk != 1 || bk != 1 {
            return Err("each side needs exactly one king".into());
        }
        b.fix_castling();
        Ok(b)
    }

    /// Drop castling rights whose king or rook is not on its home square.
    pub fn fix_castling(&mut self) {
        if self.sq[4] != KING {
            self.castling &= !(CASTLE_WK | CASTLE_WQ);
        }
        if self.sq[7] != ROOK {
            self.castling &= !CASTLE_WK;
        }
        if self.sq[0] != ROOK {
            self.castling &= !CASTLE_WQ;
        }
        if self.sq[60] != -KING {
            self.castling &= !(CASTLE_BK | CASTLE_BQ);
        }
        if self.sq[63] != -ROOK {
            self.castling &= !CASTLE_BK;
        }
        if self.sq[56] != -ROOK {
            self.castling &= !CASTLE_BQ;
        }
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                let p = self.sq[(rank * 8 + file) as usize];
                if p == 0 {
                    empty += 1;
                } else {
                    if empty > 0 {
                        let _ = write!(s, "{}", empty);
                        empty = 0;
                    }
                    s.push(piece_char(p));
                }
            }
            if empty > 0 {
                let _ = write!(s, "{}", empty);
            }
            if rank > 0 {
                s.push('/');
            }
        }
        let _ = write!(
            s,
            " {} {} {} {} {}",
            if self.white_to_move { 'w' } else { 'b' },
            self.castling_str(),
            if self.ep == NO_SQ { "-".to_string() } else { sq_name(self.ep) },
            self.halfmove,
            self.fullmove
        );
        s
    }

    pub fn castling_str(&self) -> String {
        let mut c = String::new();
        if self.castling & CASTLE_WK != 0 {
            c.push('K');
        }
        if self.castling & CASTLE_WQ != 0 {
            c.push('Q');
        }
        if self.castling & CASTLE_BK != 0 {
            c.push('k');
        }
        if self.castling & CASTLE_BQ != 0 {
            c.push('q');
        }
        if c.is_empty() {
            c.push('-');
        }
        c
    }

    pub fn king_sq(&self, white: bool) -> u8 {
        let k = if white { KING } else { -KING };
        self.sq.iter().position(|&p| p == k).map(|i| i as u8).unwrap_or(NO_SQ)
    }

    #[inline]
    fn own(&self, p: i8, white: bool) -> bool {
        if white {
            p > 0
        } else {
            p < 0
        }
    }

    /// Is `target` attacked by any piece of the given color (geometric attack)?
    pub fn attacked_by(&self, target: u8, by_white: bool) -> bool {
        self.attackers(target, by_white).next().is_some()
    }

    /// Squares of pieces of `by_white` color that attack `target` geometrically.
    pub fn attackers(&self, target: u8, by_white: bool) -> impl Iterator<Item = u8> + '_ {
        let mut out: Vec<u8> = Vec::new();
        let tf = file_of(target);
        let tr = rank_of(target);
        // pawns
        let dir: i8 = if by_white { -1 } else { 1 };
        for df in [-1i8, 1] {
            if let Some(s) = make_sq(tf + df, tr + dir) {
                let p = self.sq[s as usize];
                if p == if by_white { PAWN } else { -PAWN } {
                    out.push(s);
                }
            }
        }
        for (df, dr) in KNIGHT_D {
            if let Some(s) = make_sq(tf + df, tr + dr) {
                if self.sq[s as usize] == if by_white { KNIGHT } else { -KNIGHT } {
                    out.push(s);
                }
            }
        }
        for (df, dr) in KING_D {
            if let Some(s) = make_sq(tf + df, tr + dr) {
                if self.sq[s as usize] == if by_white { KING } else { -KING } {
                    out.push(s);
                }
            }
        }
        for (dirs, kinds) in [(&BISHOP_D, [BISHOP, QUEEN]), (&ROOK_D, [ROOK, QUEEN])] {
            for &(df, dr) in dirs.iter() {
                let mut f = tf + df;
                let mut r = tr + dr;
                while let Some(s) = make_sq(f, r) {
                    let p = self.sq[s as usize];
                    if p != 0 {
                        if self.own(p, by_white) && kinds.contains(&p.abs()) {
                            out.push(s);
                        }
                        break;
                    }
                    f += df;
                    r += dr;
                }
            }
        }
        out.into_iter()
    }

    /// Squares attacked by the piece standing on `from` (geometric, ignores pins).
    pub fn attacks_from(&self, from: u8) -> Vec<u8> {
        let p = self.sq[from as usize];
        let mut out = Vec::new();
        if p == 0 {
            return out;
        }
        let f0 = file_of(from);
        let r0 = rank_of(from);
        match p.abs() {
            PAWN => {
                let dir = if p > 0 { 1 } else { -1 };
                for df in [-1i8, 1] {
                    if let Some(s) = make_sq(f0 + df, r0 + dir) {
                        out.push(s);
                    }
                }
            }
            KNIGHT => {
                for (df, dr) in KNIGHT_D {
                    if let Some(s) = make_sq(f0 + df, r0 + dr) {
                        out.push(s);
                    }
                }
            }
            KING => {
                for (df, dr) in KING_D {
                    if let Some(s) = make_sq(f0 + df, r0 + dr) {
                        out.push(s);
                    }
                }
            }
            t => {
                let mut dirs: Vec<(i8, i8)> = Vec::new();
                if t == BISHOP || t == QUEEN {
                    dirs.extend_from_slice(&BISHOP_D);
                }
                if t == ROOK || t == QUEEN {
                    dirs.extend_from_slice(&ROOK_D);
                }
                for (df, dr) in dirs {
                    let mut f = f0 + df;
                    let mut r = r0 + dr;
                    while let Some(s) = make_sq(f, r) {
                        out.push(s);
                        if self.sq[s as usize] != 0 {
                            break;
                        }
                        f += df;
                        r += dr;
                    }
                }
            }
        }
        out
    }

    pub fn in_check(&self, white: bool) -> bool {
        let k = self.king_sq(white);
        k != NO_SQ && self.attacked_by(k, !white)
    }

    fn pseudo_moves(&self, out: &mut Vec<Move>) {
        let white = self.white_to_move;
        for from in 0..64u8 {
            let p = self.sq[from as usize];
            if p == 0 || !self.own(p, white) {
                continue;
            }
            let f0 = file_of(from);
            let r0 = rank_of(from);
            match p.abs() {
                PAWN => {
                    let dir: i8 = if white { 1 } else { -1 };
                    let start_rank = if white { 1 } else { 6 };
                    let last_rank = if white { 7 } else { 0 };
                    let push = |to: u8, out: &mut Vec<Move>| {
                        if rank_of(to) == last_rank {
                            for pr in [QUEEN, ROOK, BISHOP, KNIGHT] {
                                out.push(Move { from, to, promo: pr });
                            }
                        } else {
                            out.push(Move { from, to, promo: 0 });
                        }
                    };
                    if let Some(one) = make_sq(f0, r0 + dir) {
                        if self.sq[one as usize] == 0 {
                            push(one, out);
                            if r0 == start_rank {
                                if let Some(two) = make_sq(f0, r0 + 2 * dir) {
                                    if self.sq[two as usize] == 0 {
                                        out.push(Move { from, to: two, promo: 0 });
                                    }
                                }
                            }
                        }
                    }
                    for df in [-1i8, 1] {
                        if let Some(to) = make_sq(f0 + df, r0 + dir) {
                            let q = self.sq[to as usize];
                            if (q != 0 && !self.own(q, white)) || to == self.ep {
                                push(to, out);
                            }
                        }
                    }
                }
                KNIGHT | KING => {
                    let ds: &[(i8, i8)] = if p.abs() == KNIGHT { &KNIGHT_D } else { &KING_D };
                    for &(df, dr) in ds {
                        if let Some(to) = make_sq(f0 + df, r0 + dr) {
                            let q = self.sq[to as usize];
                            if q == 0 || !self.own(q, white) {
                                out.push(Move { from, to, promo: 0 });
                            }
                        }
                    }
                    if p.abs() == KING {
                        self.castle_moves(white, out);
                    }
                }
                t => {
                    let mut dirs: Vec<(i8, i8)> = Vec::new();
                    if t == BISHOP || t == QUEEN {
                        dirs.extend_from_slice(&BISHOP_D);
                    }
                    if t == ROOK || t == QUEEN {
                        dirs.extend_from_slice(&ROOK_D);
                    }
                    for (df, dr) in dirs {
                        let mut f = f0 + df;
                        let mut r = r0 + dr;
                        while let Some(to) = make_sq(f, r) {
                            let q = self.sq[to as usize];
                            if q == 0 {
                                out.push(Move { from, to, promo: 0 });
                            } else {
                                if !self.own(q, white) {
                                    out.push(Move { from, to, promo: 0 });
                                }
                                break;
                            }
                            f += df;
                            r += dr;
                        }
                    }
                }
            }
        }
    }

    fn castle_moves(&self, white: bool, out: &mut Vec<Move>) {
        let (kbit, qbit, ksq) = if white { (CASTLE_WK, CASTLE_WQ, 4u8) } else { (CASTLE_BK, CASTLE_BQ, 60u8) };
        if self.sq[ksq as usize] != if white { KING } else { -KING } {
            return;
        }
        if self.in_check(white) {
            return;
        }
        if self.castling & kbit != 0
            && self.sq[(ksq + 1) as usize] == 0
            && self.sq[(ksq + 2) as usize] == 0
            && !self.attacked_by(ksq + 1, !white)
            && !self.attacked_by(ksq + 2, !white)
        {
            out.push(Move { from: ksq, to: ksq + 2, promo: 0 });
        }
        if self.castling & qbit != 0
            && self.sq[(ksq - 1) as usize] == 0
            && self.sq[(ksq - 2) as usize] == 0
            && self.sq[(ksq - 3) as usize] == 0
            && !self.attacked_by(ksq - 1, !white)
            && !self.attacked_by(ksq - 2, !white)
        {
            out.push(Move { from: ksq, to: ksq - 2, promo: 0 });
        }
    }

    /// All legal moves, in the Step 4 listing order.
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut pseudo = Vec::with_capacity(64);
        self.pseudo_moves(&mut pseudo);
        let white = self.white_to_move;
        let mut legal: Vec<Move> = pseudo.into_iter().filter(|m| !self.make(m).in_check(white)).collect();
        legal.sort_by_key(listing_key);
        legal
    }

    /// Legal moves for `white` as if it were that side's turn (used for listings of both sides).
    pub fn legal_moves_for(&self, white: bool) -> Vec<Move> {
        if self.white_to_move == white {
            return self.legal_moves();
        }
        let mut b = *self;
        b.white_to_move = white;
        b.ep = NO_SQ;
        b.legal_moves()
    }

    pub fn is_capture(&self, m: &Move) -> bool {
        self.sq[m.to as usize] != 0 || self.is_en_passant(m)
    }

    pub fn is_en_passant(&self, m: &Move) -> bool {
        self.sq[m.from as usize].abs() == PAWN && m.to == self.ep && file_of(m.from) != file_of(m.to)
    }

    pub fn is_castle(&self, m: &Move) -> bool {
        self.sq[m.from as usize].abs() == KING && (file_of(m.from) - file_of(m.to)).abs() == 2
    }

    /// The piece captured by `m` (0 if none).
    pub fn captured_piece(&self, m: &Move) -> i8 {
        if self.is_en_passant(m) {
            if self.white_to_move {
                -PAWN
            } else {
                PAWN
            }
        } else {
            self.sq[m.to as usize]
        }
    }

    /// Square of the piece captured by `m` (differs from `to` for en passant).
    pub fn captured_sq(&self, m: &Move) -> u8 {
        if self.is_en_passant(m) {
            if self.white_to_move {
                m.to - 8
            } else {
                m.to + 8
            }
        } else {
            m.to
        }
    }

    /// Returns the board after `m`. `m` must be at least pseudo-legal.
    pub fn make(&self, m: &Move) -> Board {
        let mut b = *self;
        let p = b.sq[m.from as usize];
        let white = p > 0;
        let capture = self.is_capture(m);
        if self.is_en_passant(m) {
            let cs = self.captured_sq(m);
            b.sq[cs as usize] = 0;
        }
        b.sq[m.to as usize] = if m.promo != 0 { if white { m.promo } else { -m.promo } } else { p };
        b.sq[m.from as usize] = 0;
        if self.is_castle(m) {
            let (rf, rt) = if m.to > m.from { (m.from + 3, m.from + 1) } else { (m.from - 4, m.from - 1) };
            b.sq[rt as usize] = b.sq[rf as usize];
            b.sq[rf as usize] = 0;
        }
        b.ep = NO_SQ;
        if p.abs() == PAWN && (rank_of(m.to) - rank_of(m.from)).abs() == 2 {
            b.ep = (m.from + m.to) / 2;
        }
        b.fix_castling();
        b.halfmove = if p.abs() == PAWN || capture { 0 } else { self.halfmove + 1 };
        if !white {
            b.fullmove += 1;
        }
        b.white_to_move = !self.white_to_move;
        b
    }

    /// The en passant square, but only when a legal en passant capture exists.
    pub fn legal_ep(&self) -> u8 {
        if self.ep == NO_SQ {
            return NO_SQ;
        }
        if self.legal_moves().iter().any(|m| self.is_en_passant(m)) {
            self.ep
        } else {
            NO_SQ
        }
    }

    pub fn is_checkmate(&self) -> bool {
        self.in_check(self.white_to_move) && self.legal_moves().is_empty()
    }

    pub fn is_stalemate(&self) -> bool {
        !self.in_check(self.white_to_move) && self.legal_moves().is_empty()
    }

    /// King against king, or king and one minor piece against king.
    pub fn insufficient_material(&self) -> bool {
        let mut minors = 0;
        for &p in self.sq.iter() {
            match p.abs() {
                0 | KING => {}
                KNIGHT | BISHOP => minors += 1,
                _ => return false,
            }
        }
        minors <= 1
    }

    /// Standard Algebraic Notation for a legal move.
    pub fn san(&self, m: &Move) -> String {
        let p = self.sq[m.from as usize];
        let mut s = String::new();
        if self.is_castle(m) {
            s.push_str(if m.to > m.from { "O-O" } else { "O-O-O" });
        } else if p.abs() == PAWN {
            if self.is_capture(m) {
                s.push((b'a' + (m.from % 8)) as char);
                s.push('x');
            }
            s.push_str(&sq_name(m.to));
            if m.promo != 0 {
                s.push('=');
                s.push(piece_char(m.promo));
            }
        } else {
            s.push(piece_char(p.abs()));
            let others: Vec<Move> = self
                .legal_moves()
                .into_iter()
                .filter(|o| o.to == m.to && o.from != m.from && self.sq[o.from as usize] == p)
                .collect();
            if !others.is_empty() {
                let same_file = others.iter().any(|o| file_of(o.from) == file_of(m.from));
                let same_rank = others.iter().any(|o| rank_of(o.from) == rank_of(m.from));
                if !same_file {
                    s.push((b'a' + (m.from % 8)) as char);
                } else if !same_rank {
                    s.push((b'1' + (m.from / 8)) as char);
                } else {
                    s.push_str(&sq_name(m.from));
                }
            }
            if self.is_capture(m) {
                s.push('x');
            }
            s.push_str(&sq_name(m.to));
        }
        let after = self.make(m);
        if after.in_check(after.white_to_move) {
            s.push(if after.legal_moves().is_empty() { '#' } else { '+' });
        }
        s
    }

    /// Parse a move typed as from-to coordinates ("e2e4", "e7e8q") or as SAN.
    pub fn parse_move(&self, text: &str) -> Option<Move> {
        let t = text.trim();
        let legal = self.legal_moves();
        let lower = t.to_ascii_lowercase();
        if lower.len() >= 4 {
            if let (Some(f), Some(to)) = (parse_sq(&lower[0..2]), parse_sq(&lower[2..4])) {
                let promo = match lower.chars().nth(4) {
                    Some('q') => QUEEN,
                    Some('r') => ROOK,
                    Some('b') => BISHOP,
                    Some('n') => KNIGHT,
                    _ => 0,
                };
                let found = legal.iter().find(|m| {
                    m.from == f && m.to == to && (m.promo == promo || (promo == 0 && m.promo == QUEEN))
                });
                if let Some(m) = found {
                    return Some(*m);
                }
            }
        }
        let clean = |s: &str| s.trim_end_matches(&['+', '#'][..]).to_string();
        legal.into_iter().find(|m| clean(&self.san(m)) == clean(t))
    }

    pub fn ascii(&self) -> String {
        let mut s = String::new();
        for rank in (0..8).rev() {
            let _ = write!(s, "{} ", rank + 1);
            for file in 0..8 {
                let p = self.sq[(rank * 8 + file) as usize];
                s.push(if p == 0 { '.' } else { piece_char(p) });
                s.push(' ');
            }
            s.push('\n');
        }
        s.push_str("  a b c d e f g h\n");
        s
    }

    /// Position as a piece list, side to move, castling rights and en passant square.
    /// This is the key text used in listings and in the position index. It is not a FEN.
    pub fn key(&self) -> String {
        let mut s = String::new();
        for i in 0..64u8 {
            let p = self.sq[i as usize];
            if p != 0 {
                if !s.is_empty() {
                    s.push(',');
                }
                s.push(piece_char(p));
                s.push_str(&sq_name(i));
            }
        }
        let ep = self.legal_ep();
        let _ = write!(
            s,
            "|{}|{}|{}",
            if self.white_to_move { 'w' } else { 'b' },
            self.castling_str(),
            if ep == NO_SQ { "-".to_string() } else { sq_name(ep) }
        );
        s
    }

    /// Rebuild a board from `key()` text (clocks are not part of the key).
    pub fn from_key(key: &str) -> Result<Board, String> {
        let parts: Vec<&str> = key.split('|').collect();
        if parts.len() != 4 {
            return Err("a key has four parts".into());
        }
        let mut b = Board::empty();
        for item in parts[0].split(',').filter(|x| !x.is_empty()) {
            let c = item.chars().next().ok_or("empty item")?;
            let s = parse_sq(&item[1..]).ok_or("bad square in key")?;
            let t = match c.to_ascii_lowercase() {
                'p' => PAWN,
                'n' => KNIGHT,
                'b' => BISHOP,
                'r' => ROOK,
                'q' => QUEEN,
                'k' => KING,
                _ => return Err("bad piece in key".into()),
            };
            b.sq[s as usize] = if c.is_ascii_uppercase() { t } else { -t };
        }
        b.white_to_move = parts[1] == "w";
        for c in parts[2].chars() {
            b.castling |= match c {
                'K' => CASTLE_WK,
                'Q' => CASTLE_WQ,
                'k' => CASTLE_BK,
                'q' => CASTLE_BQ,
                _ => 0,
            };
        }
        b.ep = if parts[3] == "-" { NO_SQ } else { parse_sq(parts[3]).ok_or("bad ep")? };
        Ok(b)
    }

    pub fn count(&self, p: i8) -> usize {
        self.sq.iter().filter(|&&q| q == p).count()
    }
}

/// Count legal move sequences to `depth`. Writes nothing and fills no list.
pub fn perft(b: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = b.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }
    moves.iter().map(|m| perft(&b.make(m), depth - 1)).sum()
}
