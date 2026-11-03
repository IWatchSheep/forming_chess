//! Steps 4, 5 and 7: the two history files of the game being played.

use crate::board::*;
use crate::strategies::TurnInfo;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const WINDOW: usize = 5;

#[derive(Clone, Debug)]
pub struct HistoryRecord {
    pub ply: u32,
    pub move_no: u32,
    pub white: bool,
    pub mv: Move,
    pub san: String,
    pub turn: TurnInfo,
    pub before: Board,
    pub after: Board,
}

/// Every piece of both colors with its legal moves, plus the side to move,
/// castling rights and en passant square (Step 4).
pub fn listing(b: &Board) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "  side to move: {}", if b.white_to_move { "White" } else { "Black" });
    let _ = writeln!(s, "  castling rights: {}", b.castling_str());
    let ep = b.legal_ep();
    let _ = writeln!(s, "  en passant: {}", if ep == NO_SQ { "-".to_string() } else { sq_name(ep) });
    let _ = writeln!(s, "  key: {}", b.key());
    for white in [true, false] {
        let moves = b.legal_moves_for(white);
        let _ = writeln!(s, "  {} pieces:", if white { "White" } else { "Black" });
        for sq in 0..64u8 {
            let p = b.sq[sq as usize];
            if p == 0 || (p > 0) != white {
                continue;
            }
            let list: Vec<String> = moves.iter().filter(|m| m.from == sq).map(|m| m.uci()).collect();
            let _ = writeln!(
                s,
                "    {} {}: {}",
                piece_name(p),
                sq_name(sq),
                if list.is_empty() { "no legal moves".to_string() } else { list.join(" ") }
            );
        }
    }
    s
}

pub struct History {
    dir: PathBuf,
    pub white: Vec<HistoryRecord>,
    pub black: Vec<HistoryRecord>,
}

impl History {
    pub fn new(dir: &Path) -> History {
        History { dir: dir.to_path_buf(), white: Vec::new(), black: Vec::new() }
    }

    pub fn path(&self, white: bool) -> PathBuf {
        self.dir.join(if white { "white_history.txt" } else { "black_history.txt" })
    }

    /// Step 7: clear both history files at the start of a new game.
    pub fn clear(&mut self) -> io::Result<()> {
        self.white.clear();
        self.black.clear();
        fs::create_dir_all(&self.dir)?;
        self.write(true)?;
        self.write(false)
    }

    pub fn push(&mut self, rec: HistoryRecord) -> io::Result<()> {
        let white = rec.white;
        let list = if white { &mut self.white } else { &mut self.black };
        list.push(rec);
        if list.len() > WINDOW {
            list.remove(0);
        }
        self.write(white)
    }

    /// The evaluated player's previous four turns and current turn.
    pub fn window(&self, white: bool) -> Vec<TurnInfo> {
        let list = if white { &self.white } else { &self.black };
        list.iter().map(|r| r.turn.clone()).collect()
    }

    fn write(&self, white: bool) -> io::Result<()> {
        let list = if white { &self.white } else { &self.black };
        let mut s = String::new();
        let _ = writeln!(
            s,
            "{} history: previous four turns and the current turn ({} kept)",
            if white { "White" } else { "Black" },
            list.len()
        );
        for (i, r) in list.iter().enumerate() {
            let _ = writeln!(
                s,
                "\nRECORD ply={} move={} side={} move={} san={}{}",
                r.ply,
                r.move_no,
                if r.white { "White" } else { "Black" },
                r.mv.uci(),
                r.san,
                if i + 1 == list.len() { " (current turn)" } else { "" }
            );
            let _ = writeln!(s, "BEFORE");
            s.push_str(&listing(&r.before));
            let _ = writeln!(s, "AFTER");
            s.push_str(&listing(&r.after));
        }
        fs::create_dir_all(&self.dir)?;
        fs::write(self.path(white), s)
    }
}
