//! One-way stations the player pairs (`StationData::pair`, `Network::pair_of`): a station on each
//! leg of a one-way pair made one stop of a line, each run stopping at the half on its own track,
//! where on its own each is a stop served once (T-3605). Set from the station panel, picking the
//! other half on the map: only the player knows which platforms of a big station go together.

use super::lines::{edited_stops, settle_ones};
use super::*;

#[wasm_bindgen]
impl TrackApi {
    /// Pair one-way stations `a` and `b` as one stop's two halves (`Network::pair_of`), each leaving
    /// any pair it was in. A line listing both halves keeps the one its forward run stops at, and one
    /// listing only the half its way back stops at lists the other instead, so each is one stop (a
    /// pair either leaves is undone as `unpair_station` does). One undoable edit, free. Refused where
    /// the two cannot pair (`Network::pair_refusal`) or a line stopping there cannot take it.
    pub fn pair_stations(&mut self, a: u32, b: u32) -> u32 {
        let net = &self.w.net;
        if let Some(&node) = [a, b].iter().find(|&&n| net.station_data(n).is_none()) {
            return self.refuse(Issue::NoSuchNode { node });
        }
        if let Some(i) = net.pair_refusal(a, b) {
            return self.refuse(i);
        }
        if net.pair_of(a) == Some(b) {
            return 0;
        }
        self.pair_edit(vec![(a, Some(b)), (b, Some(a))])
    }

    /// Unpair station `a`: it and the station it was paired with are one-way stations of their own
    /// again (T-3605), and a line that stopped at the pair lists both, each stopped at once a round
    /// trip by the run along its track. One undoable edit, free; nothing changes for a station in
    /// no pair.
    pub fn unpair_station(&mut self, a: u32) -> u32 {
        if self.w.net.node_pair.get(a as usize).copied().flatten().is_none() {
            return 0;
        }
        self.pair_edit(vec![(a, None)])
    }

    /// The station `node` is paired with (`Network::pair_of`), else -1: a pair the track no longer
    /// allows (a leg made two-way) is none until it does again.
    pub fn station_pair(&self, node: u32) -> i32 {
        self.w.net.pair_of(node).map_or(-1, |p| p as i32)
    }

    /// The stations `node` can be paired with (`Network::pair_candidates`), nearest first, for the
    /// map's pick.
    pub fn station_pair_candidates(&self, node: u32) -> Vec<u32> {
        self.w.net.pair_candidates(node)
    }
}

impl TrackApi {
    /// The edit giving each station of `set` its pair (`StationData::pair`), any station one of them
    /// was paired with left in none, and the lines stopping at any of them listing each pair made
    /// as one stop and each pair undone as two, routed on the track as it will be. Run as one edit.
    fn pair_edit(&mut self, mut set: Vec<(u32, Option<u32>)>) -> u32 {
        let net = &self.w.net;
        for k in 0..set.len() {
            let (n, p) = set[k];
            let left = net.node_pair[n as usize].filter(|&o| Some(o) != p && net.node_pair.get(o as usize).copied().flatten() == Some(n));
            if let Some(o) = left.filter(|o| !set.iter().any(|x| x.0 == *o)) {
                set.push((o, None));
            }
        }
        // each pair once, lower node first: those made, and those that now hold and will not
        let made: Vec<[u32; 2]> = set.iter().filter_map(|&(n, p)| p.filter(|&p| n < p).map(|p| [n, p])).collect();
        let undone: Vec<[u32; 2]> = set.iter().filter_map(|&(n, p)| net.pair_of(n).filter(|&o| Some(o) != p && n < o).map(|o| [n, o])).collect();
        let mut ops: Vec<Op> = set.iter().filter_map(|&(node, pair)| net.station_data(node).map(|d| Op::Station { node, data: Some(StationData { pair, ..d }) })).collect();
        let touched: Vec<u32> = set.iter().map(|x| x.0).collect();
        // each open rail line stopping at one of them, as its runs stop now: run 0 at each stop or not,
        // and run 1's stop in each one's place
        let mut plans = vec![];
        for l in 0..net.line_count() as u32 {
            let i = l as usize;
            if !net.line_ok(l) || net.line_bus[i].is_some() || net.line_ferry[i].is_some() || closed(net.line_stops(l)) || !net.line_stops(l).iter().any(|s| touched.contains(s)) {
                continue;
            }
            let Some(d) = net.line_data(l).filter(|d| !d.path.is_empty()) else { continue };
            let nodes: Vec<u32> = std::iter::once(net.path_nodes(d.path[0]).0).chain(d.path.iter().map(|&p| net.path_nodes(p).1)).collect();
            let Some((_, stopped)) = net.match_run(&nodes, &d.stops, &net.stop_reversals(&d.path), &d.ones, &d.path) else { continue };
            let back = net.way_stops(&d.stops, &d.path, &d.ones).map_or_else(|| d.stops.clone(), |w| w.1);
            let n = d.stops.len();
            let at = |v: u32| d.stops.iter().position(|&s| s == v).filter(|&j| j > 0 && j + 1 < n);
            let mut stops = d.stops.clone();
            for &[x, y] in &made {
                match (at(x), at(y)) {
                    // both halves listed: the one run 0 stops at stays, the way back stopping at the other
                    (Some(i), Some(j)) if stopped[i] != stopped[j] => stops.retain(|&s| s != if stopped[i] { y } else { x }),
                    // the half the way back stops at: the other, which run 0 passes
                    (Some(i), None) if !stopped[i] => stops.iter_mut().filter(|s| **s == x).for_each(|s| *s = y),
                    (None, Some(j)) if !stopped[j] => stops.iter_mut().filter(|s| **s == y).for_each(|s| *s = x),
                    _ => {}
                }
            }
            for &[x, y] in &undone {
                // the half the way back stopped at goes in beside the one listed, a stop of its own
                let (lone, other) = match (at(x), at(y)) {
                    (Some(i), None) if back[i] == y => (x, y),
                    (None, Some(j)) if back[j] == x => (y, x),
                    _ => continue,
                };
                if let Some(k) = stops.iter().position(|&s| s == lone) {
                    stops.insert(k + 1, other);
                }
            }
            if stops != d.stops {
                plans.push((l, d, stops));
            }
        }
        if !plans.is_empty() {
            let inv = match self.w.sketch(Op::Batch(ops.clone()), false) {
                Ok(inv) => inv,
                Err(mut v) => return self.refuse(v.swap_remove(0)),
            };
            for (l, mut d, stops) in plans {
                let net = &self.w.net;
                // the path kept where it still passes them right; else found again (the order too,
                // as a stop edit's, unless the player set it); a line with neither is left for the
                // checks to name
                if net.check_route(l, &stops, &d.path, net.line_runs_once(l)).is_ok() {
                    d.stops = stops;
                } else if let Some((s, p)) = edited_stops(net, &stops, d.hand, &d.ones) {
                    (d.stops, d.path) = (s, p);
                } else {
                    continue;
                }
                if settle_ones(net, &mut d) {
                    ops.push(Op::Line { id: l, data: Some(d) });
                }
            }
            self.w.unsketch(inv);
        }
        self.run(Op::Batch(ops))
    }
}
