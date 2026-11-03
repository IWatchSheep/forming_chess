//! Independent check: solve king and pawn against king by plain retrograde
//! iteration, then compare every proven result in a drilled index.
use forming_chess::{archive::Archive, drill, index::{Index, Status}, normalize::{self, Cat}, board::*};
use std::collections::HashMap;
fn main() {
    let all = normalize::kpk_positions(true);
    let keys: Vec<String> = all.iter().map(|b| normalize::normalize(b).key()).collect();
    let id: HashMap<String, usize> = keys.iter().cloned().enumerate().map(|(i,k)|(k,i)).collect();
    // successor values: Some(true)=win goal, Some(false)=draw goal, None+idx = position
    enum S { Goal(bool), Pos(usize) }
    let succ: Vec<Vec<S>> = all.iter().map(|b| {
        let n = normalize::normalize(b);
        n.board.legal_moves().iter().map(|m| {
            let after = n.board.make(m);
            if m.promo != 0 {
                let r = after.legal_moves();
                if r.is_empty() { return S::Goal(after.in_check(after.white_to_move)); }
                if m.promo == BISHOP || m.promo == KNIGHT { return S::Goal(false); }
                return S::Goal(!r.iter().any(|x| x.to == m.to));
            }
            let na = normalize::normalize(&after);
            if na.cat != Cat::Kpk { return S::Goal(false); }
            S::Pos(id[&na.key()])
        }).collect()
    }).collect();
    let n = all.len();
    let mut dist: Vec<Option<u32>> = vec![None; n];
    for (i,b) in all.iter().enumerate() { let nb = normalize::normalize(b); if nb.board.legal_moves().is_empty() && nb.board.in_check(nb.board.white_to_move) && !nb.board.white_to_move { dist[i]=Some(0);} }
    loop {
        let mut changed = false;
        let snap = dist.clone();
        for i in 0..n {
            if snap[i].is_some() { continue; }
            let nb = normalize::normalize(&all[i]);
            let v: Vec<Option<u32>> = succ[i].iter().map(|s| match s { S::Goal(true)=>Some(0), S::Goal(false)=>None, S::Pos(j)=>snap[*j] }).collect();
            if v.is_empty() { continue; }
            if nb.board.white_to_move {
                if let Some(d) = v.iter().flatten().min() { dist[i]=Some(d+1); changed=true; }
            } else if v.iter().all(|x| x.is_some()) {
                dist[i]=Some(v.iter().flatten().max().unwrap()+1); changed=true;
            }
        }
        if !changed { break; }
    }
    let wins = dist.iter().filter(|d| d.is_some()).count();
    println!("true: {} positions, {} wins for the pawn side, {} draws", n, wins, n-wins);
    // drill and compare
    let games: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(20000);
    let dir = std::env::temp_dir().join("verify_kpk"); let _ = std::fs::remove_dir_all(&dir);
    let ar = Archive::new(&dir); let mut ix = Index::new();
    drill::run(&mut ix, &ar, 1, games, |_, _| {}).unwrap();
    let (mut ok_w, mut ok_d, mut bad, mut longer) = (0,0,0,0);
    for node in &ix.nodes {
        if node.cat != Cat::Kpk { continue; }
        let Some(&i) = id.get(&node.key) else { println!("unknown key {}", node.key); bad+=1; continue };
        match (node.status, dist[i]) {
            (Status::Win(d), Some(t)) => { ok_w+=1; if d < t { bad+=1; println!("too short {} {} < {}", node.key, d, t);} else if d > t { longer+=1; } }
            (Status::Win(_), None) => { bad+=1; println!("false win {}", node.key); }
            (Status::Draw, None) => ok_d+=1,
            (Status::Draw, Some(_)) => { bad+=1; println!("false draw {}", node.key); }
            _ => {}
        }
    }
    println!("after {} drill games: proven wins correct {}, proven draws correct {}, wrong {}, win distances longer than best {}", games, ok_w, ok_d, bad, longer);
}
