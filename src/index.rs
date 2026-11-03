//! Step 15: one entry per distinct position, and Step 12: how a result becomes proven.

use crate::board::*;
use crate::normalize::{self, Cat, Normalized};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Unknown,
    /// Proven win for the pawn side (or the rook side), with the distance in individual moves.
    Win(u32),
    /// Proven draw.
    Draw,
}

impl Status {
    pub fn text(&self) -> String {
        match self {
            Status::Unknown => "-".into(),
            Status::Win(d) => format!("W{}", d),
            Status::Draw => "D".into(),
        }
    }
    pub fn parse(s: &str) -> Status {
        if s == "D" {
            Status::Draw
        } else if let Some(d) = s.strip_prefix('W') {
            Status::Win(d.parse().unwrap_or(0))
        } else {
            Status::Unknown
        }
    }
    pub fn is_proven(&self) -> bool {
        !matches!(self, Status::Unknown)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub played: u64,
    pub wins: u64,
    pub draws: u64,
    pub losses: u64,
}

impl Counts {
    pub fn add(&mut self, o: &Counts) {
        self.played += o.played;
        self.wins += o.wins;
        self.draws += o.draws;
        self.losses += o.losses;
    }
    pub fn results(&self) -> u64 {
        self.wins + self.draws + self.losses
    }
    /// Wins divided by finished results (0 when there are none).
    pub fn win_rate(&self) -> f64 {
        if self.results() == 0 {
            0.0
        } else {
            self.wins as f64 / self.results() as f64
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cont {
    pub count: u64,
    pub gain: i32,
    pub loss: i32,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub idx: usize,
    pub to: usize,
    /// A goal reached by the move itself (promotion in king and pawn against king).
    pub goal: Option<Status>,
    pub counts: BTreeMap<String, Counts>,
    /// Per template, the stage pairs that past games produced: bit `d * 8 + f` is set when
    /// one game had destination stages `d` and, after its own next full turn, stages `f`.
    /// The two are never taken from different games and then combined.
    pub pairs: [u64; 20],
    /// Played continuations within one full turn: (bank, reply, next) -> count, gain, loss.
    pub conts: BTreeMap<(String, String, String), Cont>,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub key: String,
    pub cat: Cat,
    pub white_to_move: bool,
    /// Legal moves on the stored board, in the Step 4 order.
    pub legal: Vec<String>,
    pub edges: Vec<Option<usize>>,
    pub status: Status,
    pub terminal: bool,
    /// Stored positions that have a tried move into this one.
    pub preds: Vec<usize>,
}

/// The bit of one (destination stages, later stages) pair.
pub fn pair_bit(d: u8, f: u8) -> u64 {
    1u64 << ((d & 7) as u32 * 8 + (f & 7) as u32)
}

impl Edge {
    pub fn new(from: usize, idx: usize, to: usize, goal: Option<Status>) -> Edge {
        Edge { from, idx, to, goal, counts: BTreeMap::new(), pairs: [0; 20], conts: BTreeMap::new() }
    }
}

impl Node {
    pub fn fully_tried(&self) -> bool {
        self.terminal || self.edges.iter().all(|e| e.is_some())
    }
    /// In the proof family the stored board has the pawn side (or rook side) as White.
    pub fn attacker_to_move(&self) -> bool {
        self.white_to_move
    }
}

/// How a finished game ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameResult {
    WhiteWin,
    BlackWin,
    Draw,
    DrawByRule,
    Stopped,
}

impl GameResult {
    pub fn text(&self) -> &'static str {
        match self {
            GameResult::WhiteWin => "white-wins",
            GameResult::BlackWin => "black-wins",
            GameResult::Draw => "draw",
            GameResult::DrawByRule => "draw-by-rule",
            GameResult::Stopped => "stopped",
        }
    }
    pub fn parse(s: &str) -> GameResult {
        match s {
            "white-wins" => GameResult::WhiteWin,
            "black-wins" => GameResult::BlackWin,
            "draw" => GameResult::Draw,
            "draw-by-rule" => GameResult::DrawByRule,
            _ => GameResult::Stopped,
        }
    }
}

/// One saved move of a finished game.
#[derive(Clone, Debug)]
pub struct SavedMove {
    pub uci: String,
    /// The mover's template flags after the move.
    pub flags: [u8; 20],
}

#[derive(Clone, Debug)]
pub struct GameRecord {
    pub id: u64,
    pub bank: String,
    pub start: Board,
    pub moves: Vec<SavedMove>,
    pub result: GameResult,
}

#[derive(Default)]
pub struct Index {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub by_key: HashMap<String, usize>,
    pub merged: Vec<u64>,
    merged_set: HashSet<u64>,
    /// Similar-position counts for king and pawn against king: "simkey|relmove" -> bank -> counts.
    pub similar: HashMap<String, BTreeMap<String, Counts>>,
}

fn status_rank_changed(old: Status, new: Status) -> bool {
    old != new
}

impl Index {
    pub fn new() -> Index {
        Index::default()
    }

    pub fn is_merged(&self, id: u64) -> bool {
        self.merged_set.contains(&id)
    }

    pub fn lookup(&self, key: &str) -> Option<usize> {
        self.by_key.get(key).copied()
    }

    /// Find or create the node for a normalized position.
    fn node_for(&mut self, n: &Normalized) -> usize {
        let key = n.key();
        if let Some(&id) = self.by_key.get(&key) {
            return id;
        }
        let b = &n.board;
        let legal: Vec<String> = b.legal_moves().iter().map(|m| m.uci()).collect();
        let (terminal, status) = terminal_status(b, n.cat, legal.is_empty());
        let id = self.nodes.len();
        self.nodes.push(Node {
            key: key.clone(),
            cat: n.cat,
            white_to_move: b.white_to_move,
            edges: vec![None; legal.len()],
            legal,
            status,
            terminal,
            preds: Vec::new(),
        });
        self.by_key.insert(key, id);
        id
    }

    /// The value a tried move gives in a proof: its own goal, or the status of its destination.
    pub fn edge_value(&self, e: usize) -> Status {
        let edge = &self.edges[e];
        edge.goal.unwrap_or(self.nodes[edge.to].status)
    }

    /// The proof rules of Step 12 for one position, from its tried moves.
    fn evaluate(&self, id: usize) -> Status {
        let n = &self.nodes[id];
        if n.terminal || !n.cat.proof_family() {
            return n.status;
        }
        let vals: Vec<Option<Status>> = n.edges.iter().map(|e| e.map(|e| self.edge_value(e))).collect();
        let all_tried = vals.iter().all(|v| v.is_some());
        let derived = if n.attacker_to_move() {
            let best_win = vals.iter().filter_map(|v| match v {
                Some(Status::Win(d)) => Some(*d),
                _ => None,
            }).min();
            if let Some(d) = best_win {
                Status::Win(d + 1)
            } else if all_tried && vals.iter().all(|v| *v == Some(Status::Draw)) {
                Status::Draw
            } else {
                Status::Unknown
            }
        } else if vals.contains(&Some(Status::Draw)) {
            Status::Draw
        } else if all_tried && vals.iter().all(|v| matches!(v, Some(Status::Win(_)))) {
            let worst = vals.iter().filter_map(|v| match v {
                Some(Status::Win(d)) => Some(*d),
                _ => None,
            }).max().unwrap_or(0);
            Status::Win(worst + 1)
        } else {
            Status::Unknown
        };
        // A proven status is never taken back; only a win's distance can shrink.
        match (n.status, derived) {
            (Status::Draw, _) => Status::Draw,
            (Status::Win(old), Status::Win(new)) => Status::Win(old.min(new)),
            (Status::Win(old), _) => Status::Win(old),
            (Status::Unknown, d) => d,
        }
    }

    /// The backward update: re-evaluate, and when a status changes, re-evaluate
    /// every stored position with a tried move into it, until nothing changes.
    /// Returns every position that was looked at.
    fn propagate(&mut self, seeds: &[usize]) -> Vec<usize> {
        let mut queue: VecDeque<usize> = seeds.iter().copied().collect();
        let mut queued: HashSet<usize> = seeds.iter().copied().collect();
        let mut touched = Vec::new();
        while let Some(id) = queue.pop_front() {
            queued.remove(&id);
            touched.push(id);
            let old = self.nodes[id].status;
            let new = self.evaluate(id);
            if status_rank_changed(old, new) {
                self.nodes[id].status = new;
                for &p in &self.nodes[id].preds.clone() {
                    if queued.insert(p) {
                        queue.push_back(p);
                    }
                }
            }
        }
        touched
    }

    /// Does `start` reach, by tried moves, a position that still has an untried move?
    /// Returns the visited set when it does not (the position is closed).
    fn closed_region(&self, start: usize, reaches_open: &mut HashSet<usize>) -> Option<Vec<usize>> {
        let mut stack = vec![start];
        let mut seen: HashSet<usize> = HashSet::new();
        seen.insert(start);
        let mut order = Vec::new();
        while let Some(v) = stack.pop() {
            if reaches_open.contains(&v) {
                return None;
            }
            let n = &self.nodes[v];
            order.push(v);
            if n.terminal || (n.status.is_proven() && v != start) {
                // A proven position needs no further reading for closure; a proven
                // win could never be a draw, and a proven draw is already settled.
                continue;
            }
            if !n.fully_tried() {
                return None;
            }
            for e in n.edges.iter().flatten() {
                let edge = &self.edges[*e];
                if edge.goal.is_some() {
                    continue;
                }
                if seen.insert(edge.to) {
                    stack.push(edge.to);
                }
            }
        }
        Some(order)
    }

    /// The closing rule for draws, checked on the given positions and on every
    /// stored position with a tried move into one of them.
    fn close_draws(&mut self, touched: &[usize]) {
        let mut pending: Vec<usize> = touched.to_vec();
        loop {
            let mut cands: Vec<usize> = Vec::new();
            let mut seen = HashSet::new();
            for &t in &pending {
                if seen.insert(t) {
                    cands.push(t);
                }
                for &p in &self.nodes[t].preds {
                    if seen.insert(p) {
                        cands.push(p);
                    }
                }
            }
            let mut reaches_open: HashSet<usize> = HashSet::new();
            let mut newly: Vec<usize> = Vec::new();
            for c in cands {
                let n = &self.nodes[c];
                if !n.cat.proof_family() || n.status.is_proven() {
                    continue;
                }
                match self.closed_region(c, &mut reaches_open) {
                    None => {
                        reaches_open.insert(c);
                    }
                    Some(region) => {
                        for v in region {
                            if self.nodes[v].cat.proof_family() && self.nodes[v].status == Status::Unknown {
                                self.nodes[v].status = Status::Draw;
                                newly.push(v);
                            }
                        }
                    }
                }
            }
            if newly.is_empty() {
                break;
            }
            let mut seeds = Vec::new();
            for &v in &newly {
                seeds.extend(self.nodes[v].preds.iter().copied());
            }
            pending = self.propagate(&seeds);
            pending.extend(newly);
        }
    }

    /// Run the backward update and the closing rule on the given positions.
    /// Merging calls this itself; it is public so the proof rules can be checked
    /// on a small hand-made graph.
    pub fn recheck(&mut self, ids: &[usize]) {
        let touched = self.propagate(ids);
        self.close_draws(&touched);
    }

    /// Add a hand-made position (for checking the proof rules on a small graph).
    pub fn add_test_node(&mut self, key: &str, cat: Cat, white_to_move: bool, legal: usize) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node {
            key: key.to_string(),
            cat,
            white_to_move,
            legal: (0..legal).map(|i| format!("m{}", i)).collect(),
            edges: vec![None; legal],
            status: Status::Unknown,
            terminal: legal == 0,
            preds: Vec::new(),
        });
        self.by_key.insert(key.to_string(), id);
        id
    }

    /// Add a hand-made tried move between two hand-made positions.
    pub fn add_test_edge(&mut self, from: usize, idx: usize, to: usize, goal: Option<Status>) -> usize {
        let e = self.edges.len();
        self.edges.push(Edge::new(from, idx, to, goal));
        self.nodes[from].edges[idx] = Some(e);
        if !self.nodes[to].preds.contains(&from) {
            self.nodes[to].preds.push(from);
        }
        e
    }

    /// Merge one finished or stopped game (Step 15.4).
    pub fn merge(&mut self, g: &GameRecord) -> Result<(), String> {
        if self.merged_set.contains(&g.id) {
            return Ok(());
        }
        let mut boards = vec![g.start];
        let mut moves = Vec::new();
        for sm in &g.moves {
            let b = *boards.last().unwrap();
            let m = b.parse_move(&sm.uci).ok_or_else(|| format!("game {}: illegal move {}", g.id, sm.uci))?;
            moves.push(m);
            boards.push(b.make(&m));
        }
        // Check the whole game before the index changes at all: every move must be in
        // its position's legal-move listing. One bad move leaves the game out.
        let norms: Vec<Normalized> = boards.iter().map(normalize::normalize).collect();
        for i in 0..moves.len() {
            let nm = norms[i].t.mv(&moves[i]).uci();
            if !norms[i].board.legal_moves().iter().any(|x| x.uci() == nm) {
                return Err(format!("game {}: move {} is missing from its legal-move listing", g.id, nm));
            }
        }
        let mut path = Vec::new();
        for i in 0..moves.len() {
            let b = &boards[i];
            let nb = &norms[i];
            let from = self.node_for(nb);
            let to = self.node_for(&norms[i + 1]);
            let nm = nb.t.mv(&moves[i]).uci();
            let idx = self.nodes[from]
                .legal
                .iter()
                .position(|x| *x == nm)
                .ok_or_else(|| format!("game {}: move {} missing from its listing", g.id, nm))?;
            let e = match self.nodes[from].edges[idx] {
                Some(e) => e,
                None => {
                    let goal = edge_goal(nb, &moves[i], b);
                    let e = self.edges.len();
                    self.edges.push(Edge::new(from, idx, to, goal));
                    self.nodes[from].edges[idx] = Some(e);
                    if !self.nodes[to].preds.contains(&from) {
                        self.nodes[to].preds.push(from);
                    }
                    e
                }
            };
            // Whose counts: the pawn or rook side in the endgames, otherwise the side to move.
            let perspective_white = if nb.cat.folded() { !nb.t.flip } else { b.white_to_move };
            let mut add = Counts { played: 1, ..Counts::default() };
            match g.result {
                GameResult::Stopped => {}
                GameResult::Draw | GameResult::DrawByRule => add.draws = 1,
                GameResult::WhiteWin => {
                    if perspective_white {
                        add.wins = 1
                    } else {
                        add.losses = 1
                    }
                }
                GameResult::BlackWin => {
                    if perspective_white {
                        add.losses = 1
                    } else {
                        add.wins = 1
                    }
                }
            }
            self.edges[e].counts.entry(g.bank.clone()).or_default().add(&add);
            // One pair per game: the stages at the destination and, in the same game,
            // the stages after the mover's next move.
            for k in 0..20 {
                let later = if i + 2 < g.moves.len() { g.moves[i + 2].flags[k] } else { 0 };
                self.edges[e].pairs[k] |= pair_bit(g.moves[i].flags[k], later);
            }
            // The move and the played full turn after it: captures by either side, and promotions.
            let mut gain = 0;
            let mut loss = 0;
            for j in i..(i + 3).min(moves.len()) {
                let bj = &boards[j];
                let mj = &moves[j];
                let mut v = 0;
                if bj.is_capture(mj) {
                    v += piece_value(bj.captured_piece(mj));
                }
                if mj.promo != 0 {
                    v += piece_value(mj.promo) - 100;
                }
                if (j - i) % 2 == 0 {
                    gain += v;
                } else {
                    loss += v;
                }
            }
            if gain != 0 || loss != 0 {
                let reply = moves.get(i + 1).map(|m| nb.t.mv(m).uci()).unwrap_or_else(|| "-".into());
                let next = moves.get(i + 2).map(|m| nb.t.mv(m).uci()).unwrap_or_else(|| "-".into());
                let c = self.edges[e].conts.entry((g.bank.clone(), reply, next)).or_default();
                c.count += 1;
                c.gain = gain;
                c.loss = loss;
            }
            if let Some(sk) = normalize::similar_key(nb) {
                let rm = normalize::relative_move(&nb.board, &nb.t.mv(&moves[i]));
                self.similar.entry(format!("{}|{}", sk, rm)).or_default().entry(g.bank.clone()).or_default().add(&add);
            }
            path.push(from);
            path.push(to);
        }
        path.reverse();
        let mut seeds = Vec::new();
        let mut seen = HashSet::new();
        for p in path {
            if seen.insert(p) {
                seeds.push(p);
            }
        }
        let touched = self.propagate(&seeds);
        self.close_draws(&touched);
        self.merged.push(g.id);
        self.merged_set.insert(g.id);
        Ok(())
    }

    pub fn proven_counts(&self) -> (usize, usize, usize) {
        let mut w = 0;
        let mut d = 0;
        for n in &self.nodes {
            match n.status {
                Status::Win(_) => w += 1,
                Status::Draw => d += 1,
                _ => {}
            }
        }
        (self.nodes.len(), w, d)
    }

    /// Text form of the whole index.
    pub fn to_text(&self) -> String {
        let mut s = String::with_capacity(self.nodes.len() * 120);
        s.push_str("FORMING-CHESS-INDEX 2\n");
        for g in &self.merged {
            let _ = writeln!(s, "G\t{}", g);
        }
        for n in &self.nodes {
            let _ = writeln!(
                s,
                "N\t{}\t{}\t{}\t{}\t{}",
                n.key,
                n.cat.code(),
                n.status.text(),
                if n.terminal { "T" } else { "-" },
                if n.legal.is_empty() { "-".to_string() } else { n.legal.join(",") }
            );
        }
        for (i, e) in self.edges.iter().enumerate() {
            let _ = writeln!(
                s,
                "E\t{}\t{}\t{}\t{}",
                e.from,
                e.idx,
                e.to,
                e.goal.map(|g| g.text()).unwrap_or_else(|| "-".into())
            );
            let pairs: Vec<String> =
                e.pairs.iter().enumerate().filter(|(_, m)| **m != 0).map(|(k, m)| format!("{}:{:x}", k, m)).collect();
            if !pairs.is_empty() {
                let _ = writeln!(s, "P\t{}\t{}", i, pairs.join(","));
            }
            for (bank, c) in &e.counts {
                let _ = writeln!(s, "C\t{}\t{}\t{}\t{}\t{}\t{}", i, bank, c.played, c.wins, c.draws, c.losses);
            }
            for ((bank, r, nx), c) in &e.conts {
                let _ = writeln!(s, "K\t{}\t{}\t{}\t{}\t{}\t{}\t{}", i, bank, r, nx, c.count, c.gain, c.loss);
            }
        }
        let mut sims: Vec<(&String, &BTreeMap<String, Counts>)> = self.similar.iter().collect();
        sims.sort_by(|a, b| a.0.cmp(b.0));
        for (k, banks) in sims {
            for (bank, c) in banks {
                let _ = writeln!(s, "S\t{}\t{}\t{}\t{}\t{}\t{}", k, bank, c.played, c.wins, c.draws, c.losses);
            }
        }
        s
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, self.to_text())?;
        fs::rename(&tmp, path)
    }

    pub fn load(path: &Path) -> Result<Index, String> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Index::new()),
            Err(e) => return Err(e.to_string()),
        };
        Index::from_text(&text)
    }

    /// Read the index text. Every line is checked for all its fields before any is
    /// used; a bad line is an error, never a panic.
    pub fn from_text(text: &str) -> Result<Index, String> {
        let mut ix = Index::new();
        let mut lines = text.lines().enumerate();
        match lines.next() {
            None => return Ok(ix),
            Some((_, h)) if h == "FORMING-CHESS-INDEX 2" => {}
            Some((_, h)) => return Err(format!("unknown index format \"{}\"", h)),
        }
        for (n, line) in lines {
            let at = n + 1;
            let f: Vec<&str> = line.split('\t').collect();
            let want = match f[0] {
                "G" => 2,
                "N" => 6,
                "E" => 5,
                "P" => 3,
                "C" => 7,
                "K" => 8,
                "S" => 7,
                "" => continue,
                _ => continue,
            };
            if f.len() < want {
                return Err(format!("index line {}: short {} line ({} fields, need {})", at, f[0], f.len(), want));
            }
            let num = |i: usize| -> Result<u64, String> {
                f[i].parse::<u64>().map_err(|e| format!("index line {}: field {}: {}", at, i, e))
            };
            let int = |i: usize| -> Result<i32, String> {
                f[i].parse::<i32>().map_err(|e| format!("index line {}: field {}: {}", at, i, e))
            };
            match f[0] {
                "G" => {
                    let id = num(1)?;
                    ix.merged.push(id);
                    ix.merged_set.insert(id);
                }
                "N" => {
                    let legal: Vec<String> =
                        if f[5] == "-" { Vec::new() } else { f[5].split(',').map(|s| s.to_string()).collect() };
                    let id = ix.nodes.len();
                    ix.by_key.insert(f[1].to_string(), id);
                    ix.nodes.push(Node {
                        key: f[1].to_string(),
                        cat: Cat::from_code(f[2]),
                        white_to_move: f[1].contains("|w|"),
                        edges: vec![None; legal.len()],
                        legal,
                        status: Status::parse(f[3]),
                        terminal: f[4] == "T",
                        preds: Vec::new(),
                    });
                }
                "E" => {
                    let from = num(1)? as usize;
                    let idx = num(2)? as usize;
                    let to = num(3)? as usize;
                    let goal = if f[4] == "-" { None } else { Some(Status::parse(f[4])) };
                    let e = ix.edges.len();
                    ix.edges.push(Edge::new(from, idx, to, goal));
                    let node = ix.nodes.get_mut(from).ok_or_else(|| format!("index line {}: edge from unknown node", at))?;
                    *node.edges.get_mut(idx).ok_or_else(|| format!("index line {}: edge index out of range", at))? = Some(e);
                    let tn = ix.nodes.get_mut(to).ok_or_else(|| format!("index line {}: edge to unknown node", at))?;
                    if !tn.preds.contains(&from) {
                        tn.preds.push(from);
                    }
                }
                "P" => {
                    let e = num(1)? as usize;
                    let edge = ix.edges.get_mut(e).ok_or_else(|| format!("index line {}: pairs for unknown edge", at))?;
                    for part in f[2].split(',') {
                        let (k, m) = part.split_once(':').ok_or_else(|| format!("index line {}: bad pair field", at))?;
                        let k: usize = k.parse().map_err(|_| format!("index line {}: bad template number", at))?;
                        let m = u64::from_str_radix(m, 16).map_err(|_| format!("index line {}: bad pair mask", at))?;
                        *edge.pairs.get_mut(k).ok_or_else(|| format!("index line {}: template out of range", at))? = m;
                    }
                }
                "C" => {
                    let e = num(1)? as usize;
                    let c = Counts { played: num(3)?, wins: num(4)?, draws: num(5)?, losses: num(6)? };
                    ix.edges.get_mut(e).ok_or_else(|| format!("index line {}: count for unknown edge", at))?.counts.insert(f[2].to_string(), c);
                }
                "K" => {
                    let e = num(1)? as usize;
                    let c = Cont { count: num(5)?, gain: int(6)?, loss: int(7)? };
                    ix.edges
                        .get_mut(e)
                        .ok_or_else(|| format!("index line {}: continuation for unknown edge", at))?
                        .conts
                        .insert((f[2].to_string(), f[3].to_string(), f[4].to_string()), c);
                }
                "S" => {
                    let c = Counts { played: num(3)?, wins: num(4)?, draws: num(5)?, losses: num(6)? };
                    ix.similar.entry(f[1].to_string()).or_default().insert(f[2].to_string(), c);
                }
                _ => {}
            }
        }
        Ok(ix)
    }

    /// Counts of one tried move, for one bank or for all banks.
    pub fn edge_counts(&self, e: usize, bank: Option<&str>) -> Counts {
        let mut c = Counts::default();
        for (b, x) in &self.edges[e].counts {
            if bank.is_none_or(|want| want == b) {
                c.add(x);
            }
        }
        c
    }
}

/// Terminal positions and their fixed status.
fn terminal_status(b: &Board, cat: Cat, no_moves: bool) -> (bool, Status) {
    match cat {
        Cat::DeadDraw => (true, Status::Draw),
        Cat::Kqk => {
            if no_moves {
                return (true, if b.in_check(b.white_to_move) && !b.white_to_move { Status::Win(0) } else { Status::Draw });
            }
            if !b.white_to_move {
                let qsq = (0..64u8).find(|&s| b.sq[s as usize] == QUEEN).unwrap_or(0);
                let capturable = b.legal_moves().iter().any(|m| m.to == qsq);
                (true, if capturable { Status::Draw } else { Status::Win(0) })
            } else {
                (true, Status::Win(0))
            }
        }
        Cat::Kpk | Cat::Krk => {
            if no_moves {
                if b.in_check(b.white_to_move) && !b.white_to_move {
                    (true, Status::Win(0))
                } else {
                    (true, Status::Draw)
                }
            } else {
                (false, Status::Unknown)
            }
        }
        Cat::Other => (no_moves, Status::Unknown),
    }
}

/// The goal of a promotion in king and pawn against king (Step 12): stalemate is
/// checked first; a queen or rook that cannot be captured at once wins; a bishop
/// or knight is a draw.
fn edge_goal(nb: &Normalized, real_move: &Move, _real: &Board) -> Option<Status> {
    if nb.cat != Cat::Kpk || real_move.promo == 0 {
        return None;
    }
    let m = nb.t.mv(real_move);
    let after = nb.board.make(&m);
    let replies = after.legal_moves();
    if replies.is_empty() {
        return Some(if after.in_check(after.white_to_move) { Status::Win(0) } else { Status::Draw });
    }
    if m.promo == BISHOP || m.promo == KNIGHT {
        return Some(Status::Draw);
    }
    if replies.iter().any(|r| r.to == m.to) {
        return Some(Status::Draw);
    }
    Some(Status::Win(0))
}

/// Is a position, reached in a game, the end of a king-and-pawn drill by a goal?
pub fn goal_after(before: &Board, m: &Move, after: &Board) -> Option<GameResult> {
    let nb = normalize::normalize(before);
    if nb.cat == Cat::Kpk {
        if let Some(g) = edge_goal(&nb, m, before) {
            return Some(match g {
                Status::Win(_) => {
                    if nb.t.flip {
                        GameResult::BlackWin
                    } else {
                        GameResult::WhiteWin
                    }
                }
                _ => GameResult::Draw,
            });
        }
    }
    let cat = normalize::categorize(after);
    if cat == Cat::DeadDraw {
        return Some(GameResult::Draw);
    }
    if after.legal_moves().is_empty() {
        if after.in_check(after.white_to_move) {
            return Some(if after.white_to_move { GameResult::BlackWin } else { GameResult::WhiteWin });
        }
        return Some(GameResult::Draw);
    }
    None
}
