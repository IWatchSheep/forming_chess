//! Step 15.4 and 15.5: compact game files, banks, and merging into the index.

use crate::board::*;
use crate::index::{GameRecord, GameResult, Index, SavedMove};
use crate::strategies;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const GENERAL: &str = "general";
pub const DRILL: &str = "drill";

pub struct Archive {
    pub root: PathBuf,
    next: std::cell::Cell<u64>,
}

impl Archive {
    pub fn new(game_dir: &Path) -> Archive {
        let a = Archive { root: game_dir.join("ai_learning"), next: std::cell::Cell::new(0) };
        a.next.set(a.scan_next_id());
        a
    }

    pub fn index_path(&self) -> PathBuf {
        self.root.join("index.txt")
    }
    pub fn games_dir(&self) -> PathBuf {
        self.root.join("games")
    }
    pub fn bank_path(&self, bank: &str) -> PathBuf {
        self.root.join("banks").join(format!("{}.txt", bank))
    }

    /// Bank entries: (game id, file name).
    pub fn bank_entries(&self, bank: &str) -> Vec<(u64, String)> {
        let Ok(text) = fs::read_to_string(self.bank_path(bank)) else { return Vec::new() };
        text.lines()
            .filter_map(|l| {
                let mut it = l.split('\t');
                let id = it.next()?.parse().ok()?;
                let file = it.next()?.to_string();
                Some((id, file))
            })
            .collect()
    }

    pub fn banks(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(self.root.join("banks")) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if let Some(b) = name.strip_suffix(".txt") {
                    out.push(b.to_string());
                }
            }
        }
        out.sort();
        out
    }

    pub fn next_id(&self) -> u64 {
        self.next.get()
    }

    fn scan_next_id(&self) -> u64 {
        self.banks()
            .iter()
            .flat_map(|b| self.bank_entries(b))
            .map(|(id, _)| id)
            .max()
            .map_or(1, |m| m + 1)
    }

    fn add_to_bank(&self, bank: &str, id: u64, file: &str) -> io::Result<()> {
        let p = self.bank_path(bank);
        if let Some(d) = p.parent() {
            fs::create_dir_all(d)?;
        }
        let mut f = OpenOptions::new().create(true).append(true).open(p)?;
        writeln!(f, "{}\t{}", id, file)
    }

    /// Start a saved game: its file and its bank entry. A normal game has its own
    /// file. Drill games share one file per drill run (`run_file`), one block each.
    pub fn begin_game(&self, bank: &str, start: &Board, run_file: Option<&str>) -> io::Result<GameWriter> {
        let id = self.next_id();
        self.next.set(id + 1);
        let file = match run_file {
            Some(f) => f.to_string(),
            None => format!("game_{}.txt", id),
        };
        fs::create_dir_all(self.games_dir())?;
        let path = self.games_dir().join(&file);
        let mut w = GameWriter { path, id, bank: bank.to_string(), text: String::new(), buffered: run_file.is_some() };
        w.text.push_str(&format!("GAME START id={} bank={} start={}\n", id, bank, start.key()));
        if !w.buffered {
            fs::write(&w.path, &w.text)?;
            w.text.clear();
        }
        self.add_to_bank(bank, id, &file)?;
        Ok(w)
    }

    /// Every game block in the banks, in id order.
    pub fn all_games(&self) -> Vec<(GameRecord, bool)> {
        let mut out = Vec::new();
        let mut seen_files = std::collections::HashSet::new();
        for bank in self.banks() {
            for (_, file) in self.bank_entries(&bank) {
                if !seen_files.insert(file.clone()) {
                    continue;
                }
                if let Ok(text) = fs::read_to_string(self.games_dir().join(&file)) {
                    out.extend(parse_games(&text));
                }
            }
        }
        out.sort_by_key(|(g, _)| g.id);
        out
    }

    /// A game left without an end block (the program was killed) is marked
    /// stopped on the next start. Complete lines are not changed. A line cut off
    /// in the middle is not a move: it is dropped, and the game before it is stopped.
    pub fn mark_unfinished_stopped(&self) -> io::Result<Vec<u64>> {
        let mut stopped = Vec::new();
        let mut checked = std::collections::HashSet::new();
        for bank in self.banks() {
            for (_, file) in self.bank_entries(&bank) {
                // Many drill games share one file: read each file once.
                if !checked.insert(file.clone()) {
                    continue;
                }
                let path = self.games_dir().join(&file);
                let Ok(text) = fs::read_to_string(&path) else { continue };
                let complete = complete_lines(&text);
                if complete.len() < text.len() {
                    OpenOptions::new().write(true).open(&path)?.set_len(complete.len() as u64)?;
                }
                if let Some((g, false)) = parse_games(complete).last() {
                    let mut f = OpenOptions::new().append(true).open(&path)?;
                    writeln!(f, "GAME STOPPED")?;
                    stopped.push(g.id);
                }
            }
        }
        Ok(stopped)
    }

    /// Merge every ended game that the index does not hold yet, in id order. A game
    /// that cannot be merged is reported and left out; the games after it still merge.
    pub fn catch_up_report(&self, ix: &mut Index) -> (usize, Vec<String>) {
        let mut n = 0;
        let mut errors = Vec::new();
        for (g, finished) in self.all_games() {
            if finished && !ix.is_merged(g.id) {
                match ix.merge(&g) {
                    Ok(()) => n += 1,
                    Err(e) => errors.push(e),
                }
            }
        }
        (n, errors)
    }

    pub fn catch_up(&self, ix: &mut Index) -> Result<usize, String> {
        Ok(self.catch_up_report(ix).0)
    }

    /// Delete nothing; build a fresh index from the game files.
    pub fn rebuild_report(&self) -> (Index, Vec<String>) {
        let mut ix = Index::new();
        let (_, errors) = self.catch_up_report(&mut ix);
        (ix, errors)
    }

    pub fn rebuild(&self) -> Result<Index, String> {
        Ok(self.rebuild_report().0)
    }

    /// Load the index and merge the games it lacks. An unreadable index is reported
    /// and built again from the game files, so the program keeps running.
    pub fn open_index(&self) -> (Index, usize, Vec<String>) {
        let mut notes = Vec::new();
        let mut ix = match Index::load(&self.index_path()) {
            Ok(ix) => ix,
            Err(e) => {
                notes.push(format!("Could not read the index ({}). Building it again from the saved games.", e));
                Index::new()
            }
        };
        let (n, errors) = self.catch_up_report(&mut ix);
        for e in errors {
            notes.push(format!("Left out of the index: {}", e));
        }
        (ix, n, notes)
    }
}

/// The text up to and including its last newline: a line cut off in the middle is dropped.
fn complete_lines(text: &str) -> &str {
    if text.ends_with('\n') {
        text
    } else {
        &text[..text.rfind('\n').map_or(0, |i| i + 1)]
    }
}

/// Writes one saved game. Normal games are written line by line; drill games are
/// written as one block when they end.
pub struct GameWriter {
    pub path: PathBuf,
    pub id: u64,
    pub bank: String,
    text: String,
    buffered: bool,
}

impl GameWriter {
    pub fn add_move(&mut self, ply: u32, before: &Board, m: &Move, flags: &[u8; 20]) -> io::Result<()> {
        if self.buffered {
            // Drill records: the move and the mover's template flags only. The
            // position and its legal moves are stored once, in the index.
            let line = format!("m\t{}\t{}\n", m.uci(), strategies::flags_to_sparse(flags));
            return self.write(&line);
        }
        let legal: Vec<String> = before.legal_moves().iter().map(|x| x.uci()).collect();
        let line = format!(
            "M\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            ply,
            if before.white_to_move { "w" } else { "b" },
            m.uci(),
            before.san(m),
            strategies::flags_to_text(flags),
            before.key(),
            legal.join(",")
        );
        self.write(&line)
    }

    fn write(&mut self, s: &str) -> io::Result<()> {
        if self.buffered {
            self.text.push_str(s);
            Ok(())
        } else {
            let mut f = OpenOptions::new().append(true).open(&self.path)?;
            f.write_all(s.as_bytes())
        }
    }

    pub fn finish(mut self, result: GameResult) -> io::Result<()> {
        let end = if result == GameResult::Stopped {
            "GAME STOPPED\n".to_string()
        } else {
            format!("GAME END result={}\n", result.text())
        };
        self.write(&end)?;
        if self.buffered {
            let mut f = OpenOptions::new().create(true).append(true).open(&self.path)?;
            f.write_all(self.text.as_bytes())?;
        }
        Ok(())
    }
}

/// Parse the blocks of a game file. The flag says whether the block has an end.
pub fn parse_games(text: &str) -> Vec<(GameRecord, bool)> {
    let mut out: Vec<(GameRecord, bool)> = Vec::new();
    for line in complete_lines(text).lines() {
        if let Some(rest) = line.strip_prefix("GAME START ") {
            let mut id = 0;
            let mut bank = GENERAL.to_string();
            let mut start = Board::start();
            for part in rest.split(' ') {
                if let Some(v) = part.strip_prefix("id=") {
                    id = v.parse().unwrap_or(0);
                } else if let Some(v) = part.strip_prefix("bank=") {
                    bank = v.to_string();
                } else if let Some(v) = part.strip_prefix("start=") {
                    if let Ok(b) = Board::from_key(v) {
                        start = b;
                    }
                }
            }
            out.push((GameRecord { id, bank, start, moves: Vec::new(), result: GameResult::Stopped }, false));
        } else if let Some(rest) = line.strip_prefix("M\t") {
            if let Some((g, false)) = out.last_mut() {
                let f: Vec<&str> = rest.split('\t').collect();
                if f.len() >= 5 {
                    // A move that is not in the legal-move listing on its own line makes the
                    // whole game unmergeable: "????" is never a legal move, so the merge refuses it.
                    let listed = f.get(6).map_or(true, |l| l.split(',').any(|x| x == f[2]));
                    let uci = if listed { f[2].to_string() } else { "????".to_string() };
                    g.moves.push(SavedMove { uci, flags: strategies::flags_from_text(f[4]) });
                }
            }
        } else if let Some(rest) = line.strip_prefix("m\t") {
            if let Some((g, false)) = out.last_mut() {
                let mut it = rest.split('\t');
                if let (Some(uci), Some(fl)) = (it.next(), it.next()) {
                    g.moves.push(SavedMove { uci: uci.to_string(), flags: strategies::flags_from_sparse(fl) });
                }
            }
        } else if let Some(rest) = line.strip_prefix("GAME END result=") {
            if let Some((g, fin)) = out.last_mut() {
                g.result = GameResult::parse(rest.trim());
                *fin = true;
            }
        } else if line.starts_with("GAME STOPPED") {
            if let Some((g, fin)) = out.last_mut() {
                g.result = GameResult::Stopped;
                *fin = true;
            }
        }
    }
    out
}
