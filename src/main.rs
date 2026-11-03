//! Console program: Steps 1-3, the in-game commands, perft, drill and rebuild-index.

use forming_chess::archive::{Archive, GameWriter, GENERAL};
use forming_chess::board::*;
use forming_chess::evaluate::{list1, list2};
use forming_chess::history::{History, HistoryRecord};
use forming_chess::index::{GameRecord, GameResult, Index, SavedMove};
use forming_chess::lists34::{list3, list4, RowKind};
use forming_chess::strategies::{self, Ctx, TurnInfo, DESCRIPTIONS, NAMES};
use forming_chess::drill;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const PRESETS: [(&str, &str, &str); 6] = [
    ("start", START_FEN, "The standard starting position"),
    ("kpk-win", "3k4/8/3K4/3P4/8/8/8/8 w - - 0 1", "King and pawn against king: White's king on the sixth rank in front of its pawn"),
    ("kpk-draw", "7k/8/7K/7P/8/8/8/8 w - - 0 1", "King and pawn against king: an h-pawn with the lone king in the h8 corner"),
    ("kpk-black", "8/8/8/3p4/3k4/8/8/3K4 b - - 0 1", "King and pawn against king with Black as the pawn side"),
    ("krk", "8/8/8/4k3/8/8/8/R3K3 w - - 0 1", "King and rook against king"),
    ("kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", "A busy middlegame"),
];

struct App {
    archive: Archive,
    index: Index,
    board: Board,
    start: Board,
    history: History,
    ply: u32,
    writer: Option<GameWriter>,
    saved_moves: Vec<SavedMove>,
    seen: HashMap<String, u32>,
    over: Option<GameResult>,
    bank: Option<String>,
    windows: [Vec<TurnInfo>; 2],
}

fn prompt(text: &str) -> Option<String> {
    print!("{}", text);
    let _ = io::stdout().flush();
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim().to_string()),
    }
}

fn side_name(white: bool) -> &'static str {
    if white {
        "White"
    } else {
        "Black"
    }
}

impl App {
    fn new(dir: PathBuf) -> Result<App, String> {
        let archive = Archive::new(&dir);
        let stopped = archive.mark_unfinished_stopped().map_err(|e| e.to_string())?;
        if !stopped.is_empty() {
            println!("Marked {} unfinished saved game(s) as stopped: {:?}", stopped.len(), stopped);
        }
        let (index, merged, notes) = archive.open_index();
        for n in &notes {
            println!("{}", n);
        }
        if merged > 0 || !notes.is_empty() {
            index.save(&archive.index_path()).map_err(|e| e.to_string())?;
            println!("Merged {} finished game(s) into the position index.", merged);
        }
        let history = History::new(&dir);
        Ok(App {
            archive,
            index,
            board: Board::start(),
            start: Board::start(),
            history,
            ply: 0,
            writer: None,
            saved_moves: Vec::new(),
            seen: HashMap::new(),
            over: None,
            bank: None,
            windows: [Vec::new(), Vec::new()],
        })
    }

    /// Stop a saved game that has not ended (Step 15.4).
    fn stop_current(&mut self) {
        if self.writer.is_some() && self.over.is_none() {
            self.end_game(GameResult::Stopped, false);
        }
    }

    fn new_game(&mut self, b: Board) {
        self.stop_current();
        self.board = b;
        self.start = b;
        self.ply = 0;
        self.over = None;
        self.saved_moves.clear();
        self.seen.clear();
        self.seen.insert(b.key(), 1);
        self.windows = [Vec::new(), Vec::new()];
        if let Err(e) = self.history.clear() {
            println!("Could not clear the history files: {}", e);
        }
        println!("\nSave this game's information to the permanent AI learning files?\nY — Yes\nN — No");
        let ans = prompt("> ").unwrap_or_default();
        self.writer = None;
        if ans.eq_ignore_ascii_case("y") {
            match self.archive.begin_game(GENERAL, &b, None) {
                Ok(w) => {
                    println!("This game is saved as game {}.", w.id);
                    self.writer = Some(w);
                }
                Err(e) => println!("Could not start the game file: {}", e),
            }
        } else {
            println!("This game will not be saved.");
        }
        self.show_board();
    }

    fn show_board(&self) {
        println!("\n{}", self.board.ascii());
        println!("{} to move.", side_name(self.board.white_to_move));
    }

    fn end_game(&mut self, result: GameResult, announce: bool) {
        if announce {
            println!("Game over: {}.", result.text());
        }
        self.over = Some(result);
        if let Some(w) = self.writer.take() {
            let id = w.id;
            if let Err(e) = w.finish(result) {
                println!("Could not finish the game file: {}", e);
                return;
            }
            let rec = GameRecord {
                id,
                bank: GENERAL.to_string(),
                start: self.start,
                moves: self.saved_moves.clone(),
                result,
            };
            match self.index.merge(&rec) {
                Ok(()) => {
                    if let Err(e) = self.index.save(&self.archive.index_path()) {
                        println!("Could not save the index: {}", e);
                    } else {
                        let (p, w, d) = self.index.proven_counts();
                        println!("Game {} merged into the index: {} positions, {} proven wins, {} proven draws.", id, p, w, d);
                    }
                }
                Err(e) => println!("Could not merge game {}: {}", id, e),
            }
        }
    }

    /// Steps 1-3: select a piece, choose a destination, confirm.
    fn select(&mut self, sq: u8) {
        if self.over.is_some() {
            println!("The game is over. Type new, load or preset to start another.");
            return;
        }
        let p = self.board.sq[sq as usize];
        if p == 0 {
            println!("There is no piece on {}.", sq_name(sq));
            return;
        }
        if (p > 0) != self.board.white_to_move {
            println!("The {} on {} belongs to the side that is not to move.", piece_name(p), sq_name(sq));
            return;
        }
        let moves: Vec<Move> = self.board.legal_moves().into_iter().filter(|m| m.from == sq).collect();
        if moves.is_empty() {
            println!("The {} on {} cannot currently make a legal move.", piece_name(p), sq_name(sq));
            return;
        }
        let list: Vec<String> = moves.iter().map(|m| format!("{} ({})", sq_name(m.to), self.board.san(m))).collect();
        println!("Legal moves for the {} on {}: {}", piece_name(p), sq_name(sq), list.join(", "));
        let Some(dest) = prompt("Destination square (Enter to cancel): ") else { return };
        let Some(to) = parse_sq(&dest) else {
            println!("No move made.");
            return;
        };
        let mut choices: Vec<Move> = moves.into_iter().filter(|m| m.to == to).collect();
        if choices.is_empty() {
            println!("{} is not a legal destination. No move made.", sq_name(to));
            return;
        }
        let m = if choices.len() > 1 {
            // A promotion: only q, r, b or n is taken. Anything else asks again.
            let t = loop {
                let Some(ans) = prompt("Promote to (q, r, b, n): ") else {
                    println!("No move made.");
                    return;
                };
                match ans.to_ascii_lowercase().as_str() {
                    "q" => break QUEEN,
                    "r" => break ROOK,
                    "b" => break BISHOP,
                    "n" => break KNIGHT,
                    _ => println!("Please enter q, r, b or n."),
                }
            };
            choices.retain(|m| m.promo == t);
            choices[0]
        } else {
            choices[0]
        };
        let ans = prompt(&format!("Confirm {}? (Y/N): ", self.board.san(&m))).unwrap_or_default();
        if !ans.eq_ignore_ascii_case("y") {
            println!("Move cancelled. Nothing on the board changed.");
            return;
        }
        self.play(&m);
    }

    fn play(&mut self, m: &Move) {
        let before = self.board;
        let after = before.make(m);
        let white = before.white_to_move;
        self.ply += 1;
        let turn = TurnInfo { white, from: m.from, to: m.to, piece: before.sq[m.from as usize], captured: before.captured_piece(m) };
        let rec = HistoryRecord {
            ply: self.ply,
            move_no: before.fullmove,
            white,
            mv: *m,
            san: before.san(m),
            turn: turn.clone(),
            before,
            after,
        };
        if let Err(e) = self.history.push(rec) {
            println!("Could not write the history file: {}", e);
        }
        let w = &mut self.windows[if white { 0 } else { 1 }];
        w.push(turn);
        if w.len() > forming_chess::history::WINDOW {
            w.remove(0);
        }
        let flags = strategies::flags_all(&Ctx { b: &after, white, window: w });
        if let Some(wr) = self.writer.as_mut() {
            if let Err(e) = wr.add_move(self.ply, &before, m, &flags) {
                println!("Could not write the game file: {}", e);
            }
        }
        self.saved_moves.push(SavedMove { uci: m.uci(), flags });
        println!("Played {}.", before.san(m));
        self.board = after;
        let rep = {
            let c = self.seen.entry(after.key()).or_insert(0);
            *c += 1;
            *c
        };
        self.show_board();
        if after.is_checkmate() {
            self.end_game(if white { GameResult::WhiteWin } else { GameResult::BlackWin }, true);
        } else if after.is_stalemate() {
            println!("Stalemate.");
            self.end_game(GameResult::Draw, true);
        } else if after.insufficient_material() {
            println!("Neither side can checkmate.");
            self.end_game(GameResult::Draw, true);
        } else if rep >= 3 {
            println!("The same position has occurred three times.");
            self.end_game(GameResult::DrawByRule, true);
        } else if after.halfmove >= 100 {
            println!("Fifty moves without a pawn move or a capture.");
            self.end_game(GameResult::DrawByRule, true);
        }
    }

    fn window(&self, white: bool) -> Vec<TurnInfo> {
        self.history.window(white)
    }

    fn show_lists(&self, white: bool) {
        let w = self.window(white);
        println!("\nList 1 — Forming strategies for {} (present game only)", side_name(white));
        let rows = list1(&self.board, white, &w);
        if rows.is_empty() {
            println!("  (none)");
        }
        for r in rows {
            println!("  {:>3}  {:<20} {}", r.score, strategies::stage_name(r.score), r.name);
        }
        println!("\nList 2 — Captures for those strategies");
        if self.board.white_to_move != white {
            println!("  (only made when it is {}'s turn)", side_name(white));
            return;
        }
        let rows = list2(&self.board, white, &w);
        if rows.is_empty() {
            println!("  (none)");
        }
        for r in rows {
            println!(
                "  {:<8} {} {} takes {} {}  gain {} loss {} net {}  [{}]",
                r.san,
                piece_name(r.piece),
                sq_name(r.from),
                piece_name(r.victim),
                sq_name(r.victim_sq),
                r.exchange.gain,
                r.exchange.loss,
                r.exchange.net(),
                r.strategy
            );
        }
    }

    fn bank_label(&self) -> String {
        match &self.bank {
            Some(b) => format!("bank {}", b),
            None => "all banks".into(),
        }
    }

    fn show_list3(&self) {
        let l = list3(&self.index, &self.board, self.bank.as_deref());
        println!("\nList 3 — Past games after each legal move ({}, {})", self.bank_label(), l.cat.code());
        if l.cat.proof_family() {
            println!("  Counts are for the pawn side (or the rook side). {} is to move.", if l.pawn_side_to_move { "The pawn side" } else { "The lone king" });
        } else {
            println!("  Counts are for the side to move.");
        }
        if !l.position_known {
            println!("  This position is not in the index yet, so every move is never tried.");
        }
        for r in &l.rows {
            let status = match &r.kind {
                RowKind::ProvenWin(n) => format!("proven win in {}", n),
                RowKind::ProvenDraw => "proven draw".into(),
                RowKind::Tried => "tried".into(),
                RowKind::Similar => "never tried here; similar".into(),
                RowKind::NeverTried => "never tried".into(),
            };
            let counts = if matches!(r.kind, RowKind::NeverTried) {
                String::new()
            } else {
                format!("  played {} W{} D{} L{}", r.counts.played, r.counts.wins, r.counts.draws, r.counts.losses)
            };
            println!("  {:<8} {}{}", r.san, status, counts);
            if !r.notes.is_empty() {
                let notes: Vec<String> = r.notes.iter().map(|(n, s)| format!("{} {}", n, s)).collect();
                println!("           strategies: {}", notes.join(", "));
            }
        }
    }

    fn show_list4(&self) {
        let rows = list4(&self.index, &self.board, self.bank.as_deref());
        println!("\nList 4 — Captures and promotions that past games played after each move ({})", self.bank_label());
        if rows.is_empty() {
            println!("  (none)");
        }
        for r in rows {
            println!(
                "  {:<8} reply {:<8} next {:<8} played {}  gain {} loss {} net {}",
                r.san, r.reply, r.next, r.count, r.gain, r.loss, r.net()
            );
        }
    }

    fn show_status(&self) {
        let (p, w, d) = self.index.proven_counts();
        println!("Positions stored: {}. Proven wins: {}. Proven draws: {}.", p, w, d);
        println!("Finished games merged: {}. Tried moves: {}.", self.index.merged.len(), self.index.edges.len());
        for b in self.archive.banks() {
            println!("  bank {}: {} game(s)", b, self.archive.bank_entries(&b).len());
        }
        println!("Lists 3 and 4 read: {}.", self.bank_label());
        if let Some(w) = &self.writer {
            println!("This game is being saved as game {}.", w.id);
        }
    }

    fn show_archive(&self) {
        let games = self.archive.all_games();
        if games.is_empty() {
            println!("No saved games yet.");
            return;
        }
        let shown = games.len().min(30);
        println!("Saved games (last {} of {}):", shown, games.len());
        for (g, fin) in games.iter().skip(games.len() - shown) {
            println!(
                "  game {:>6}  bank {:<8} moves {:>3}  {}",
                g.id,
                g.bank,
                g.moves.len(),
                if *fin { g.result.text() } else { "in progress" }
            );
        }
    }

    fn help(&self) {
        println!(
            "Commands:
  e2 (any square)        select a piece, see its legal moves, enter a destination, confirm with Y or N
  lists [w|b]            List 1 and List 2
  list3                  List 3 for the side to move (from past games)
  list4                  List 4 for those moves (from past games)
  bank [name|all]        choose which bank's counts Lists 3 and 4 show
  strategies             the 20 templates;  strategy <n>  one template in full
  status                 stored positions, proven positions and banks
  archive                saved games
  new | load <fen> | presets | preset <id>
  resign | draw | board | moves | fen | help | quit"
        );
    }

    fn run(&mut self) {
        self.help();
        self.new_game(self.board);
        loop {
            let Some(line) = prompt("\n> ") else {
                self.stop_current();
                return;
            };
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let cmd = parts[0].to_ascii_lowercase();
            match cmd.as_str() {
                "quit" | "exit" => {
                    self.stop_current();
                    return;
                }
                "help" => self.help(),
                "board" => self.show_board(),
                "fen" => println!("{}", self.board.to_fen()),
                "moves" => {
                    let ms: Vec<String> = self.board.legal_moves().iter().map(|m| self.board.san(m)).collect();
                    println!("{} legal moves: {}", ms.len(), ms.join(" "));
                }
                "lists" => {
                    let white = match parts.get(1) {
                        Some(&"w") => true,
                        Some(&"b") => false,
                        _ => self.board.white_to_move,
                    };
                    self.show_lists(white);
                }
                "list3" => self.show_list3(),
                "list4" => self.show_list4(),
                "bank" => {
                    match parts.get(1) {
                        None | Some(&"all") => self.bank = None,
                        Some(b) => self.bank = Some(b.to_string()),
                    }
                    println!("Lists 3 and 4 now read {}.", self.bank_label());
                }
                "strategies" => {
                    for (i, n) in NAMES.iter().enumerate() {
                        println!("  {:>2}. {}", i + 1, n);
                    }
                }
                "strategy" => {
                    let n: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    if (1..=20).contains(&n) {
                        let d = DESCRIPTIONS[n - 1];
                        println!("Strategy {}: {}\n  Foundation: {}\n  Progress: {}\n  Completion: {}", n, NAMES[n - 1], d[0], d[1], d[2]);
                    } else {
                        println!("Give a number from 1 to 20.");
                    }
                }
                "status" => self.show_status(),
                "archive" => self.show_archive(),
                "new" => self.new_game(Board::start()),
                "load" => {
                    let fen = parts[1..].join(" ");
                    match Board::from_fen(&fen) {
                        Ok(b) => self.new_game(b),
                        Err(e) => println!("Could not load that position: {}", e),
                    }
                }
                "presets" => {
                    for (id, _, d) in PRESETS.iter() {
                        println!("  {:<10} {}", id, d);
                    }
                }
                "preset" => match parts.get(1).and_then(|id| PRESETS.iter().find(|p| p.0 == *id)) {
                    Some((_, fen, _)) => self.new_game(Board::from_fen(fen).expect("preset")),
                    None => println!("Unknown preset. Type presets to see them."),
                },
                "resign" => {
                    if self.over.is_none() {
                        let white = self.board.white_to_move;
                        println!("{} resigns.", side_name(white));
                        self.end_game(if white { GameResult::BlackWin } else { GameResult::WhiteWin }, true);
                    }
                }
                "draw" => {
                    if self.over.is_none() {
                        self.end_game(GameResult::Draw, true);
                    }
                }
                _ => {
                    if let Some(sq) = parse_sq(&cmd) {
                        self.select(sq);
                    } else {
                        println!("Unknown command. Type help.");
                    }
                }
            }
        }
    }
}

fn usage() {
    println!(
        "Usage:
  forming_chess [--dir <folder>] [--fen \"<fen>\"]
  forming_chess [--dir <folder>] perft <depth> [fen]
  forming_chess [--dir <folder>] drill [seed] [games]
  forming_chess [--dir <folder>] rebuild-index"
    );
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut dir = PathBuf::from("game_files");
    let mut fen: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--dir" && i + 1 < args.len() {
            dir = PathBuf::from(&args[i + 1]);
            args.drain(i..i + 2);
        } else if args[i] == "--fen" && i + 1 < args.len() {
            fen = Some(args[i + 1].clone());
            args.drain(i..i + 2);
        } else {
            i += 1;
        }
    }
    match args.first().map(|s| s.as_str()) {
        Some("perft") => {
            let depth: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let b = match args.get(2) {
                Some(_) => Board::from_fen(&args[2..].join(" ")),
                None => Ok(Board::start()),
            };
            match b {
                Ok(b) => {
                    let mut total = 0;
                    for d in 1..=depth {
                        let n = perft(&b, d);
                        total += n;
                        println!("Individual move {}: {} sequences", d, n);
                    }
                    println!("Total: {} nodes", total);
                }
                Err(e) => println!("Bad FEN: {}", e),
            }
        }
        Some("drill") => {
            let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let games: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
            let archive = Archive::new(&dir);
            let _ = archive.mark_unfinished_stopped();
            let (mut ix, _, notes) = archive.open_index();
            for n in &notes {
                println!("{}", n);
            }
            println!("Drill: seed {}, {} game(s).", seed, games);
            let res = drill::run(&mut ix, &archive, seed, games, |g, ix| {
                if g % 1000 == 0 {
                    let (p, w, d) = ix.proven_counts();
                    println!("  {} games: {} positions, {} proven wins, {} proven draws", g, p, w, d);
                }
            });
            match res {
                Ok(r) => {
                    let (p, w, d) = ix.proven_counts();
                    println!(
                        "Done: {} games ({} pawn-side wins, {} draws at a goal, {} draws by rule).\nIndex: {} positions, {} proven wins, {} proven draws.",
                        r.games, r.pawn_wins, r.draws, r.draws_by_rule, p, w, d
                    );
                }
                Err(e) => println!("Drill stopped: {}", e),
            }
        }
        Some("rebuild-index") => {
            let archive = Archive::new(&dir);
            let (ix, errors) = archive.rebuild_report();
            for e in &errors {
                println!("Left out of the index: {}", e);
            }
            if let Err(e) = ix.save(&archive.index_path()) {
                println!("Could not save: {}", e);
            } else {
                let (p, w, d) = ix.proven_counts();
                println!("Rebuilt from {} games: {} positions, {} proven wins, {} proven draws.", ix.merged.len(), p, w, d);
            }
        }
        Some("help") | Some("--help") | Some("-h") => usage(),
        Some(other) => {
            println!("Unknown command: {}", other);
            usage();
        }
        None => {
            let mut app = match App::new(dir) {
                Ok(a) => a,
                Err(e) => {
                    println!("Could not start: {}", e);
                    return;
                }
            };
            if let Some(f) = fen {
                match Board::from_fen(&f) {
                    Ok(b) => app.board = b,
                    Err(e) => println!("Bad FEN, using the start position: {}", e),
                }
            }
            app.run();
        }
    }
}
