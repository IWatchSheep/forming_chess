# Forming Strategies Chess

A console chess program that follows `rules/rules.txt`. `rules/rules.txt` is the specification; this README explains how to use the program and how it behaves.

- **List 1 and List 2** come from the present game only: the five-turn window and the 20 strategy templates.
- **List 3 and List 4** come from past games, through a position index that stores each distinct position once.
- **No look-ahead.** There is no future search and no future file.
- **King and pawn against king gets solved** by drill games, using the proof rules in Step 12.

No outside crates. `rust-version = "1.83"`.

## Build and run

```
cargo build --release
cargo run --release
```

| Command | What it does |
|---|---|
| `cargo run --release` | Play. Files go to `./game_files`. |
| `cargo run --release -- --dir <folder>` | Put the files in another folder. |
| `cargo run --release -- --fen "<fen>"` | Start from a position. The FEN selects the board. It is not a stored future. |
| `cargo run --release -- perft <depth> [fen]` | Count legal move sequences, to check move generation. Writes nothing and fills no list. |
| `cargo run --release -- drill [seed] [games]` | Play both sides of seeded king-and-pawn starts and store the games. Default seed 1, default 1000 games. Each seed continues where it stopped last time. |
| `cargo run --release -- rebuild-index` | Build the index again from the saved games. |

In RustRover: **File → Open**, choose the folder with `Cargo.toml`, and click the Run arrow next to `fn main` in `src/main.rs`. To run the tests, right-click the `tests` folder and choose **Run 'All Tests'**.

### Commands during a game

| Command | What it does |
|---|---|
| a square (`e2`), then a destination, then Y or N | Steps 1-3: select, move, confirm |
| q, r, b or n | For a promotion, choose the new piece before confirming. Any other answer, including an empty line, asks again. End of input cancels the move. |
| `lists [w\|b]` | List 1 and List 2 |
| `list3` | List 3 for the side to move |
| `list4` | List 4 for those moves |
| `bank [name\|all]` | Which bank's counts Lists 3 and 4 show |
| `strategies`, `strategy <n>` | The 20 templates, or one of them in full |
| `status` | Stored positions, proven positions, banks |
| `archive` | Saved games |
| `new`, `load <fen>`, `presets`, `preset <id>` | Another game or a reference position |
| `resign`, `draw`, `board`, `moves`, `fen`, `help`, `quit` | |

At the start of every game (startup, `new`, `load`, `preset`) the program asks whether to save. **Y** stores a compact record. **N** stores nothing. The drill does not ask.

Presets: `start`, `kpk-win`, `kpk-draw` (h-pawn, king in the h8 corner), `kpk-black` (Black has the pawn), `krk`, `kiwipete`.

## The four lists

| List | Made from | What it answers |
|---|---|---|
| 1 | The present game's four past turns and present turn, and the 20 templates | Which strategies are forming in this game |
| 2 | List 1, on the board being played | Which captures those strategies have now |
| 3 | The position index, one row per legal move | What past games did after this move, or that the move was never tried |
| 4 | Captures and promotions that past games actually played after a List 3 move | The value of what happened, not a guessed exchange |

- **List 1** uses only the scores 100, 60 and 30. From the standard start it contains Central Pawn Occupation and King Shelter, each at 30.
- **List 3** shows each move as a proven win in N moves, a proven draw, tried before (with win, draw and loss counts), or never tried. In king and pawn against king, a never-tried move may show counts from a *similar* position. A similar row is never proof.
- **Scores 45 and 75 on List 3** come from one game's own pair of stages: the stages the move led to, and the stages after that same game's next full turn. Stages from two different games are never joined into a higher score. When games disagree, the row shows the highest score any one game earned.
- **List 4** shows each different continuation as its own row, with how many times it was played. A lost pawn is −100 and a promotion to a queen is +800.

## Tests

```
cargo test --release
```

37 tests, running in a few seconds:

| Test file | What it checks |
|---|---|
| `tests/perft.rs` | Move generation against published counts: the start position to five moves, plus three standard test positions |
| `tests/lists12.rs` | Lists 1 and 2, the Step 14.3 capture examples, the history files |
| `tests/index.rs` | The tests listed in Step 15.8, scores from one game's pair, damaged files and indexes, and a proof check against a retrograde solution |

The proof check drills 4,000 king-and-pawn games and compares every proven result with a separate retrograde solution of the endgame. The test fails if any proof is wrong. `cargo run --release --example verify_kpk -- <games>` runs the same check after any number of drill games.

## Measured result: the endgame gets solved

One run of `drill 1 1500000` on a 2-core machine:

| Drill games | Time | Proven wins | Proven draws | h-pawn corner (`kpk-draw`) |
|---|---|---|---|---|
| 200,000 | 2.5 min | 76,834 | 16,968 | not proven |
| 1,000,000 | 12.6 min | 110,259 | 36,340 | not proven |
| 1,250,000 | 15.4 min | 110,763 | 54,381 | proven draw |
| 1,350,000 | 16.4 min | **111,282** | **54,394** | proven draw |

- **The whole king-and-pawn endgame was solved** after about 1.33 million games. The proven results (111,282 wins and 54,394 draws among the 165,676 stored positions) match a separate retrograde solution exactly.
- **Not every move needs trying.** Only 114,903 positions had every legal move tried, because a won position needs only one tried winning move.
- **Most corner draws came from the closing rule.** The draw count jumped from 36,340 to 54,381 once the last circling regions were fully tried.
- **Disk at that point:** about 465 MB of drill game files, a 128 MB index, and 34 MB of bank lists.

## Files

```
game_files/
  white_history.txt, black_history.txt     the two working files (Steps 4, 5, 7)
  ai_learning/
    index.txt                              the position index (Step 15)
    games/game_<id>.txt                    one saved normal game
    games/drill_run_<id>.txt               the games of one drill run, one block each
    banks/general.txt, banks/drill.txt     which games belong to which bank
    drill_progress.txt                     where each drill seed has reached
```

- **The index can be deleted at any time.** The next start, or `rebuild-index`, builds it again from the game files, with the same result.
- **Index format.** The index file begins with `FORMING-CHESS-INDEX 2`. An index with an older header, or one that cannot be read, is rebuilt from the saved games, and the program says so and keeps running. A short or damaged line is reported with its line number.
- **Sizes.** A drill game is stored as one short line per move: the move and the mover's template flags, about 260 bytes a game. The position and its legal moves are stored once, in the index. A saved normal game also writes the position and the legal moves on each line, as Step 15.4 asks. The index stops growing once every position has been seen.

### Damaged files and stopped games

- **A game stopped without a result** (quit, or a new game) counts its moves as tried and adds nothing to the win, draw and loss counts.
- **If the program is killed** during a saved game, the next start marks that game stopped.
- **A line cut off in the middle** is dropped, the file is cut back to its last complete line, and that game is marked stopped.
- **A game is added to the index only when every move in it is accepted.** Each move is checked against the listing on its line and against that position's own legal moves. One bad move leaves that whole game out, and the games after it are still merged.

## Source

| File | Purpose |
|---|---|
| `src/board.rs` | Board, legal moves in the Step 4 order, SAN, check, mate, draws, perft |
| `src/strategies.rs` | The 20 templates (Step 14.4) |
| `src/exchange.rs` | Capture values (Step 14.3) |
| `src/evaluate.rs` | Lists 1 and 2 |
| `src/history.rs` | The two history files |
| `src/normalize.rs` | Position keys, color flip and mirror (Step 15.3), similar positions |
| `src/index.rs` | The index, the proof rules, the backward update, the closing rule |
| `src/lists34.rs` | Lists 3 and 4 |
| `src/archive.rs` | Game files, banks, stopped and damaged games, rebuild |
| `src/drill.rs` | The drill (Step 15.6) |
| `src/main.rs` | The console program |

## Choices the rules leave open

These are the choices this program makes where `rules/rules.txt` does not say:

1. **Central pawns.** For Central Pawn Break, a "central pawn" is a pawn on d4, e4, d5 or e5. With that reading, the starting List 1 is exactly Central Pawn Occupation and King Shelter, each at 30.
2. **List 4 counts the move itself.** It covers the move plus the full turn after it, so a move that is itself a promotion shows +800.
3. **Closing rule.** When checking whether a position is closed, a position that is already proven counts as settled, and the program does not look past it. This only lets a draw be proven sooner. The proof check against the retrograde solution confirms it never proves a wrong result.
4. **Other goals.**
   - Checkmating the lone king is also a win.
   - A king and queen against king position is a win when the defender is not stalemated and cannot take the queen at once.
5. **Not folded when castling rights remain.** A position that still has castling rights is never flipped or mirrored.
6. **Drill length and progress.** `drill [seed] [games]` plays `games` games, 1000 by default. The next run with the same seed continues the start sequence where the last one stopped.

## Rust version

The code is written for Rust 1.83: edition 2021, no outside crates, no newer standard-library calls, and Clippy's `incompatible_msrv` check reports nothing. It was built and tested with Rust 1.97, because 1.83 itself could not be downloaded where it was written. If 1.83 finds a problem, it will show up as a compile error, not as a change in behaviour.
