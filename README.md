# Forming Strategies Chess

**Plan only. Not implemented.** This README and `rules/rules.txt` are the text to agree before any code changes. `rules/rules.txt` is the specification. This file says the same thing in the order a reader meets it.

List 3 and List 4 are built from past games. A finished game already contains its own future: the moves that come after a saved move are that future. No future moves are stored. List 3 and List 4 do not look ahead.

The old way searched three full turns ahead. From the starting position that is 124,132,536 nodes, and a middlegame is larger. That search is retired. The counts stay here so they are not treated as a target.

## The position index

When a game finishes, each position it played is merged into one shared index, keyed by the position. A repeated position updates the existing entry. The index grows with the number of different positions, not the number of games.

King and pawn against king, counted with White's king, White's pawn on ranks 2 through 7, and Black's king, no castling and no en passant, kings not adjacent, and the side not to move not in check:

| Set | Positions |
|---|---|
| All legal positions, both sides to move | 331,352 |
| The same set with pawn files e–h reflected onto a–d | 165,676 |

A position where Black has the pawn is stored by flipping the board so the pawn side is White. Moves are mapped back to the real board when a list is shown. At about eight legal moves per position, the full index is on the order of 1.3 million move entries, and it stops growing once every position has been seen.

King and rook against king uses the same kind of index. This plan does not give a count for it.

Any other position, such as a middlegame, uses an exact key: every piece and its square, the side to move, the castling rights and the en passant square, with no flip and no mirror. The key is built from the Step 4 listing, not from a FEN.

Each stored position holds a compact record:

- the normalized position and the side to move
- every legal move, in a fixed order
- for each move a finished game has tried: how often, the wins, draws, and losses (for the pawn side, the side with the rook, or, in any other position, the side to move), and for each strategy the pairs of destination stages and stages after that same game's next full turn, and the distance if that line is proven
- the proven status of this position, when it has one
- the stored positions that have a tried move into this one, so a proof can flow backward

The legal moves are the Step 4 listing. Origin squares run from a1 through h8, and for each origin the destinations run from a1 through h8. Promotions run queen, rook, bishop, knight. In the index this order is counted on the stored board, after the flip and the mirror, so a position and its mirror image explore in the same order. A listed move with a played count of zero has never been tried. That is what lets the archive prove a result instead of only counting wins.

The index file begins with the header FORMING-CHESS-INDEX 2. An older index is rejected and rebuilt from the saved game files. A short line returns an error that names the line number, and that index is rebuilt the same way. The program says so and keeps running.

The two history files still exist for the game being played. They are not copied into the index. No future-search file is written.

## Proving a result

Proof is bookkeeping on positions the index already holds. The program does not generate a move that is missing from a stored legal-move listing. When a game finishes, the positions that game played are updated from the last move backward. Then every stored position with a tried move into a position whose status changed is updated too, and so on, until nothing changes. A proof found in a later game therefore reaches positions played only in earlier games.

Goal for king and pawn against king:

- The pawn is captured. Draw.
- Stalemate. Draw. This is checked first, including right after a promotion.
- The pawn promotes to a queen or a rook, the defender is not stalemated, and the new piece cannot be captured on the next move. Win for the pawn side. Distance 0.
- The pawn promotes to a bishop or a knight. Draw.

Goal for king and rook against king: checkmate wins for the side with the rook. Stalemate is a draw. The rook being captured is a draw. The rules below are the same with that side in place of the pawn side.

Distance is the number of individual moves until the goal. The pawn side takes the fewest. The lone king's best defence is the most.

- Pawn side to move: proven **win** if any tried move leads to a proven win. Distance is one plus the smallest such distance.
- Lone king to move: proven **win** for the pawn side only if every legal move has been tried and all of them lead to proven wins. Distance is one plus the largest such distance.
- Pawn side to move: proven **draw** if every legal move has been tried and none leads to a proven win, and every one leads to a proven draw.
- Lone king to move: proven **draw** if any tried move leads to a proven draw.

**Closing rule for draws.** The lone king can never win, so every position is a win for the pawn side or a draw. Some draws never end in a capture or a stalemate: with an h-pawn against a king in the h8 corner, the pawn side can shuffle forever. A position is *closed* when it and every stored position reachable from it by tried moves have every legal move tried. A closed position that is still not a proven win is a proven draw. King and rook against king uses the same rule. The rule is checked after every backward update, on every position the update touched and every position leading into one of them, and whenever a game tries a position's last untried move.

Until then the position is not proven. Win, draw, and loss counts are kept anyway. Counts are not proof.

## The four lists

| List | Made from | What it answers |
|---|---|---|
| 1 | The present game's four past turns and present turn, and the 20 templates | Which strategies are forming in this game |
| 2 | List 1, on the board being played | Which captures those strategies have now |
| 3 | The position index, one row per legal move | What past games did after this move, or that the move was never tried |
| 4 | Exchanges past games actually played within one full turn after a List 3 move | The value of what happened, not a guessed exchange |

List 1 does not read the index. From the standard start it contains **Central Pawn Occupation** (taking the center) and **King Shelter**, each scoring 30. Scores on List 1 are only 100, 60, and 30.

### List 3

For each legal move from the current position, look up that position and that move. No look-ahead is done. A row shows:

- **Status:** proven win in N moves, proven draw, tried before (win, draw, and loss counts), or never tried.
- **Strategies:** the stages of the 20 templates at the position that move led to in past games. In this endgame that is mainly Promotion and Passed-Pawn Advance.
- **Scores 45 and 75:** only from one game's own pair of destination stages and the stages after that game's next full turn. One full turn is the opponent's reply plus the original side's next move. Score 45 means that game already had the foundation, and progress happened in that played turn. Score 60 means the pair has the foundation and the progress. Score 75 means that game already had the foundation and the progress, and completion happened in that played turn. One game with foundation and progress, and another with foundation and then completion a turn later, show 60 and do not show 75. A game that earns 75 by itself still shows 75. When games disagree, the row shows the highest score one game earned.

Sort when the pawn side is to move: proven wins first, fastest first; then moves tried before, by win rate; then never-tried moves; proven draws last. The lone king's side uses the reverse order. In any other position, such as a middlegame, counts are for the side to move, and tried moves sort by that side's win rate, ahead of never-tried moves.

If this exact move was never played from this exact position, and the position is king and pawn against king, the row may be filled from similar positions and is labeled **similar**. Similar means the same signed distances of both kings from the pawn, the same pawn rank, the same answer to whether the pawn is on the a-file or the h-file, and the same side to move. A similar row is never proof. No similar rule exists for other material, so there a never-seen move stays never tried.

With no past games, every legal move is "never tried". List 1 is still made.

### List 4

List 4 records captures by either side, and promotions, that really happened within one full turn after the move. Values are the Step 14.3 values:

- pawn lost: −100
- promotion to a queen: +800

In this endgame, a move that let the defender take the pawn sorts to the bottom. A move that led to promotion sorts to the top. If past games played different continuations after the same move, each one is its own row with how many times it was played. A move with no played capture and no played promotion adds no row. With no past games, List 4 is empty. A similar-position exchange is labeled similar and sorts after exact-position exchanges.

## The drill

drill

drill <seed>

The default seed is 1. Starts are taken by the pawn's rank, seventh rank first, down to the second, so positions near promotion are proven first. Within a rank, the seed sets the order, and the same seed repeats it. The drill plays both sides and does not ask whether to save. Finished games go to the drill bank and into the shared index, as compact records.

Choice of move for the pawn side:

1. The shortest proven win, if one exists.
2. Otherwise the first never-tried legal move.
3. Otherwise the tried move with the best win rate for the pawn side.

Choice of move for the lone king:

1. A proven draw, if one exists.
2. Otherwise the first never-tried legal move.
3. Otherwise, when every move has been tried and all are proven losses, the longest one.
4. Otherwise the tried move with the lowest win rate for the pawn side.

The lone king tries its untried moves before a move already known to lose, because a lone-king position is proven only after every one of its moves has been tried.

A drill game ends at a goal, at the third repetition of a position, or after 100 individual moves with no pawn move and no capture. The last two are stored as "draw by rule": they add to the draw counts and prove nothing.

The drill never treats a similar row as proof. While a never-tried legal move exists, that move is played.

Once every position reachable from a start is proven, that start is solved. The top row of List 3 is then the perfect move. Opposition and the key squares come from the past games.

Proving the whole endgame takes many drill games. About 1.3 million moves each need one try, and a game tries at most about one new move per move played, so at least about 45,000 games are needed, probably many more. Open middlegames rarely repeat a position, so there List 3 shows exact past positions when they exist and proves nothing.

## Commands

There is no program in this folder yet. When there is, the commands are:

| Command | What it does |
|---|---|
| cargo run --release | Play. History files go to ./game_files. |
| cargo run --release -- --dir <folder> | Put those files in another folder. |
| cargo run --release -- --fen "<fen>" | Start from a position. The FEN selects the board. It is not a stored future. |
| cargo run --release -- perft <depth> [fen] | Count legal sequences. Writes nothing and fills no list. |
| cargo run --release -- drill [seed] | Play both sides of a seeded king-and-pawn start and store the game. |

During a game:

| Command | What it does |
|---|---|
| a square, then a destination, then Y or N | Select, move, confirm |
| q, r, b, or n, then Y or N | Choose the promotion piece, then confirm. Any other answer, including an empty line, asks again. End of input cancels the move |
| lists [w|b] | List 1 and List 2 |
| list3 | List 3 for the side to move |
| list4 | List 4 for those moves |
| strategies, strategy <n> | The 20 templates, or one of them |
| status | How many positions are stored, and how many of them are proven |
| archive | Finished games in the banks |
| new, load, presets, preset <id> | Another game or a reference position |
| resign, draw, board, moves, fen, help, quit | |

search, future, and wait are not commands.

At the start of a normal game the program asks whether to save. Y stores a compact record when the game ends. N stores nothing. The drill does not ask.

A stopped game has no result. Its moves still count as tried, and it adds nothing to the win, draw and loss counts. If the program is killed during a saved game, the next start marks that game stopped. Drill games are appended to one shared file for the run. A final line with no newline is dropped, and the file is cut back to its last complete line. Later games in the bank are still merged. A game is added to the index only when every move line in it is accepted. Each move is checked against the listing on its line and against that position's own legal moves, so one bad move leaves that whole game out. A short index line names its line number. An index that cannot be read, including a header older than FORMING-CHESS-INDEX 2, is rebuilt from the saved games, and the program keeps running. Games saved by the older six-move program can be merged from their history sections; their future-search sections are not read.

There is one shared index. Counts are kept per bank (general and drill). Proven statuses are shared, because a proof does not depend on who played the moves. A task may select one bank, which changes the counts shown on List 3 and List 4 but not the proven statuses.

## Scores and piece values

List 1:

| Condition | Stage | Score |
|---|---|---|
| Completion met now | Fully developed | 100 |
| Foundation and progress now | Partially developed | 60 |
| Foundation only, or progress only | Beginning to form | 30 |

Scores 45 and 75 are not used on List 1. On List 3 they are the highest score earned by one game's paired stages, as the List 3 section says.

| Piece | Value |
|---|---|
| Pawn | 100 |
| Knight | 320 |
| Bishop | 330 |
| Rook | 500 |
| Queen | 900 |
| King | no capture value (10,000 only inside Fork and Skewer) |

Net value is expected gain minus potential loss. Checkmate adds 100,000 on List 2. Promotion adds the new piece's value minus 100.

## The 20 templates

| # | Strategy | Foundation | Progress | Completion |
|---|---|---|---|---|
| 1 | Central Pawn Occupation | A friendly pawn is on the d- or e-file. | A friendly pawn is on d4 or e4 (White) / d5 or e5 (Black). | Both central squares hold friendly pawns, and both are defended. |
| 2 | Central Pawn Break | The opponent has a central pawn, and a friendly pawn is on a file c–f. | A friendly pawn attacks an opposing central pawn. | The turn window contains a pawn capturing a pawn on the c-, d-, e- or f-file. |
| 3 | Kingside Pawn Storm | At least two friendly pawns on the f-, g- or h-file, at least one has moved, and the king is outside those files. | A participating pawn reaches relative rank 2. | Two participating pawns reach relative rank 3 or higher, and the king is on a file a–d. |
| 4 | Queenside Minority Attack | Fewer pawns than the opponent on the a-, b- and c-files, with at least one there. | A friendly queenside pawn reaches relative rank 2. | A participating pawn reaches relative rank 3, and an opposing pawn on files a–d is isolated or doubled. |
| 5 | King Shelter | King on e1/e8 with a castling right, or already on c1, g1, c8 or g8. | The king has castled, or at most one piece is between the king and a castling rook. | King on a castled square, not attacked, and its three shelter pawns unmoved. |
| 6 | Opposite-Side Castling Attack | Friendly king on the home rank on files a–c or f–h; opposing king on the other wing. | A friendly pawn on the opposing king's wing reaches relative rank 2. | That pawn reaches relative rank 3, and a friendly rook or queen is on a half-open file on that wing. |
| 7 | Fianchetto | The b- or g-pawn has advanced one square, or a friendly bishop is on b2, g2, b7 or g7. | A friendly bishop is on one of those squares. | The bishop attacks at least one of d4, d5, e4, e5. |
| 8 | Open-File Control | At least one file has no friendly pawn, and a friendly rook or queen remains. | A friendly rook or queen is on a file with no friendly pawn. | A fully open file has more friendly major pieces than opposing ones. |
| 9 | Seventh-Rank Invasion | A friendly rook or queen exists, and at least one file has no friendly pawn. | A friendly major piece reaches relative rank 5. | A friendly rook or queen reaches the opponent's second rank and attacks an opposing piece or pawn. |
| 10 | Passed-Pawn Advance | A friendly pawn has no opposing pawn ahead on its file or either adjacent file. | The passed pawn reaches relative rank 3. | The passed pawn reaches relative rank 5 or higher and is defended. |
| 11 | Isolated-Pawn Attack | An opposing pawn has no friendly neighbouring pawn on an adjacent file. | A friendly piece attacks it, or a friendly rook or queen shares its file. | Attacks on the isolated pawn exceed its defences. |
| 12 | Backward-Pawn Pressure | An opposing pawn's advance square is attacked by a friendly pawn, not defended by an opposing pawn, and on a file with no friendly pawn. | A friendly rook or queen is on that pawn's file. | That major piece stays on the file, and the advance square is occupied or attacked. |
| 13 | Knight Outpost | A square on relative ranks 3–6 is empty or holds a friendly knight, is defended by a friendly pawn, and can no longer be attacked by an opposing pawn. | A friendly knight occupies or attacks it. | A friendly knight occupies the outpost. |
| 14 | Bishop Pair | The player has two bishops; the opponent has fewer than two. | At least one friendly bishop has left the back rank. | At least one file c–f has no pawns, and a friendly bishop has three squares of unobstructed travel. |
| 15 | Pin | A friendly bishop, rook or queen lines up with an opposing front piece and a more valuable rear piece or king. | The line to the front piece is unobstructed. | The rear piece is the king, or a second friendly piece also attacks the front piece. |
| 16 | Discovered Attack | Exactly one friendly piece stands between a friendly sliding piece and an opposing non-pawn. | The intervening piece has a legal move off the line. | That move captures or gives check, while the slider still attacks the original target. |
| 17 | Fork | A piece's square or a legal move attacks two opposing non-pawns. | Two opposing targets are attacked now. | Both targets meet the value conditions. For a sliding piece, targets qualify only if they are kings, undefended, or worth at least as much as the attacker. |
| 18 | Skewer | A friendly slider lines up with two opposing pieces, the nearer worth at least as much as the farther. | The line to the front piece is clear, and it is the king or strictly more valuable than the rear piece. | The front piece is the king, or worth more than the slider while the rear piece is undefended or worth at least as much as the slider. |
| 19 | Major-Piece Battery | Two friendly rooks or queens share a rank or file, at most two pieces between them, and at least one has moved. | At most one piece between them. | The line between them is clear, and they share a half-open file or one attacks an opposing piece along that line. |
| 20 | Promotion | A friendly pawn reaches relative rank 4 or higher. | The pawn reaches relative rank 5 or higher. | The pawn has a legal promotion move. A pawn blocked on the seventh rank stays at progress until it can legally promote. |

## Agreed points

1. No future search and no future files. The future of a saved move is the later moves of that same finished game.
2. One index entry per distinct position. King and pawn against king is 331,352 positions, or 165,676 after mirrors.
3. List 1 is the present game only. List 3 and List 4 are the index.
4. Proof comes from tried and untried legal moves, not from a search and not from win counts alone.
5. The drill explores never-tried moves and stores compact records in its own bank. The lone king tries untried moves before a known loss. Drill games end at a goal, a threefold repetition, or 100 moves with no pawn move or capture.
6. The first game lists every move as never tried and leaves List 4 empty.
7. Proofs flow backward to every stored position that leads into a changed position. Draws that go round in circles are proven by the closing rule.
8. Proven statuses are shared by all banks. Counts are kept per bank.
9. A 45 or a 75 is the best score one game earned on that move. Stages from two games are not joined into a higher score.
10. A promotion answer other than queen, rook, bishop, or knight asks again. End of input cancels the move. Drill games are appended to one shared file. A damaged line is reported, that game is left out or marked stopped, an unreadable index is rebuilt from the saved games, and later games are still merged.