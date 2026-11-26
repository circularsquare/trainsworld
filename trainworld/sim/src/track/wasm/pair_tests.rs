//! One-way stations the player pairs (`pairs.rs`): a station on each leg of a one-way pair made one
//! stop, each run stopping at the half on its own track.

use super::oneleg_tests::{new_line, pair, pair_wide};
use super::*;

/// The half of `x` and `y` run 0 of a line from west to east stops at: the one whose trains run east.
fn east_half(a: &TrackApi, x: u32, y: u32) -> (u32, u32) {
    if a.w.net.leg_heading(x).unwrap().cos() > 0.0 { (x, y) } else { (y, x) }
}

/// Line `l` lists `stop` once and its runs stop there at `fwd` (run 0) and `back` (run 1), no stop passed:
/// two stations, each a stop with its half dot the way its run's trains run there, both stops to the stop
/// editor.
fn assert_one_stop(a: &TrackApi, l: u32, stop: u32, fwd: u32, back: u32, what: &str) {
    let net = &a.w.net;
    assert!(!net.line_broken[l as usize] && net.check_line(l).is_ok(), "{what}: the line runs, {:?}", net.check_line(l));
    let ls = &a.w.svc.lines[l as usize];
    assert!(ls.ok, "{what}: the service builds");
    let n = ls.stops.len();
    let k = ls.stops.iter().position(|&s| s == stop).unwrap_or_else(|| panic!("{what}: {stop} a stop of {:?}", ls.stops));
    assert!(!ls.stops.contains(&if stop == fwd { back } else { fwd }), "{what}: one stop, {:?}", ls.stops);
    assert_eq!(ls.stop_node(0, k), Some(fwd), "{what}: run 0 at its own half");
    assert_eq!(ls.stop_node(1, n - 1 - k), Some(back), "{what}: run 1 at the other");
    assert!(!ls.runs[0].passes(k) && !ls.runs[1].passes(n - 1 - k), "{what}: stopped at both ways");
    let ways = a.line_stop_ways(l);
    for half in [fwd, back] {
        let dot: Vec<f64> = ways.chunks(2).filter(|c| c[0] as u32 == half).map(|c| c[1]).collect();
        assert_eq!(dot, vec![net.leg_heading(half).unwrap()], "{what}: {half}'s half dot, {ways:?}");
    }
    let tiers = a.stop_tiers(l);
    for half in [fwd, back] {
        assert!(tiers.chunks(3).any(|c| c[0] as u32 == half && c[1] == 0.0), "{what}: {half} a stop to the editor");
    }
    for r in 0..2 {
        assert!(net.path_reversals(&ls.runs[r].segs).is_empty(), "{what}: run {r} turns back");
    }
}

/// The side-by-side case (T-3605's, 40 m apart), on both running sides: a line clicked onto one
/// half, both, or only the half its way back stops at, each lists one stop once the two are
/// paired, run 0 stopping at its half and run 1 at the other; pairing is free and undoable.
#[test]
fn paired_halves_are_one_stop() {
    for right_hand in [true, false] {
        for clicked in 0..3 {
            let (mut a, [p, q, n, s]) = pair(right_hand, 20.0);
            let (e, w) = east_half(&a, n, s);
            let what = format!("right hand {right_hand}, clicked {clicked}");
            let l = new_line(&mut a, &[p, q]);
            let add: &[u32] = match clicked { 0 => &[e], 1 => &[e, w], _ => &[w] };
            for &x in add {
                assert_eq!(a.add_stop(l, x), 0, "{what}: {}", a.issues());
            }
            assert_eq!(a.w.net.line_stops(l).len(), 2 + add.len(), "{what}: the T-3605 stops");
            let before = a.w.net.line_stops(l).to_vec();
            let cash = a.cash();
            assert_eq!(a.pair_stations(n, s), 0, "{what}: {}", a.issues());
            assert_eq!((a.station_pair(n), a.station_pair(s)), (s as i32, n as i32), "{what}");
            assert_eq!(a.w.net.line_stops(l), &[p, e, q][..], "{what}: the half run 0 stops at");
            assert_one_stop(&a, l, e, e, w, &what);
            assert_eq!(a.line_back_stops(l), vec![p, w, q], "{what}: run 1 at the other half");
            assert!((a.cash() - cash).abs() < 1e-9, "{what}: free, {cash} to {}", a.cash());
            assert_eq!(a.undo(), 0, "{what}: {}", a.issues());
            assert_eq!((a.station_pair(n), a.station_pair(s)), (-1, -1), "{what}: undone");
            assert_eq!(a.w.net.line_stops(l), &before[..], "{what}: the line as it was");
        }
    }
}

/// A line the other way round (east to west) keeps the other half; a click on either half of a pair
/// adds the one its run 0 stops at, for a line either way round.
#[test]
fn a_click_on_either_half_adds_the_one_run_0_stops_at() {
    for right_hand in [true, false] {
        let (mut a, [p, q, n, s]) = pair(right_hand, 20.0);
        let (e, w) = east_half(&a, n, s);
        assert_eq!(a.pair_stations(n, s), 0, "{}", a.issues());
        for (ends, fwd, back) in [([p, q], e, w), ([q, p], w, e)] {
            for x in [n, s] {
                let what = format!("right hand {right_hand}, from {}, clicked {x}", ends[0]);
                let l = new_line(&mut a, &ends);
                assert_eq!(a.add_stop(l, x), 0, "{what}: {}", a.issues());
                assert_eq!(a.w.net.line_stops(l), &[ends[0], fwd, ends[1]][..], "{what}");
                assert_one_stop(&a, l, fwd, fwd, back, &what);
            }
        }
    }
}

/// Unpaired again, a line that stopped at the pair lists both halves, each served once by the run
/// along its track (T-3605), and is undone back to one stop.
#[test]
fn unpaired_halves_are_two_stops_again() {
    for right_hand in [true, false] {
        let (mut a, [p, q, n, s]) = pair(right_hand, 20.0);
        let (e, w) = east_half(&a, n, s);
        let what = format!("right hand {right_hand}");
        assert_eq!(a.pair_stations(n, s), 0, "{}", a.issues());
        let l = new_line(&mut a, &[p, e, q]);
        assert_one_stop(&a, l, e, e, w, &what);
        assert_eq!(a.unpair_station(w), 0, "{what}: {}", a.issues());
        assert_eq!((a.station_pair(n), a.station_pair(s)), (-1, -1), "{what}");
        let stops = a.w.net.line_stops(l).to_vec();
        assert!(stops.len() == 4 && stops.contains(&e) && stops.contains(&w), "{what}: both halves, {stops:?}");
        let ls = &a.w.svc.lines[l as usize];
        let m = ls.stops.len();
        for (h, run) in [(e, 0), (w, 1)] {
            let k = ls.stops.iter().position(|&x| x == h).unwrap();
            let j = if run == 0 { k } else { m - 1 - k };
            assert!(!ls.runs[run].passes(j) && ls.runs[1 - run].passes(m - 1 - j), "{what}: {h} by run {run} only");
        }
        assert_eq!(a.undo(), 0, "{what}: {}", a.issues());
        assert_one_stop(&a, l, e, e, w, &format!("{what}, undone"));
        assert_eq!(a.unpair_station(p), 0, "{what}: a station in no pair");
    }
}

/// What cannot pair is refused, naming why: a station that is no one-way station, two whose trains run
/// the same way, two too far apart to change between on foot.
#[test]
fn pairs_that_cannot_be_are_refused() {
    let (mut a, [p, _, n, s]) = pair(true, 20.0);
    assert_ne!(a.pair_stations(n, p), 0);
    assert!(a.issue_kinds().contains("PairNotOneWay"), "{}", a.issue_kinds());
    assert_ne!(a.pair_stations(n, n), 0);
    assert!(a.issue_kinds().contains("PairNotOpposite"), "{}", a.issue_kinds());
    assert_eq!(a.station_pair_candidates(n), vec![s]);
    assert!(a.station_pair_candidates(p).is_empty());
    let (mut far, _, ns, ss) = pair_wide(true, 20.0, &[-600.0], &[600.0]);
    assert_ne!(far.pair_stations(ns[0], ss[0]), 0);
    assert!(far.issue_kinds().contains("PairTooFar"), "{}", far.issue_kinds());
    assert!(far.station_pair_candidates(ns[0]).is_empty());
}

/// Her Sol: two platforms each way, each with its partner the other way, each pair a line's. Each
/// line's way back stops at its own pair's other half, never the other pair's, whichever halves the
/// player put together.
#[test]
fn two_pairs_side_by_side_keep_their_own_halves() {
    for right_hand in [true, false] {
        for crossed in [false, true] {
            let (mut a, ends, ns, ss) = pair_wide(right_hand, 20.0, &[-120.0, 120.0], &[-120.0, 120.0]);
            let what = format!("right hand {right_hand}, crossed {crossed}");
            let pairs = if crossed { [(ns[0], ss[1]), (ns[1], ss[0])] } else { [(ns[0], ss[0]), (ns[1], ss[1])] };
            for &(x, y) in &pairs {
                assert_eq!(a.pair_stations(x, y), 0, "{what}: {}", a.issues());
            }
            for &(x, y) in &pairs {
                let (e, w) = east_half(&a, x, y);
                let l = new_line(&mut a, &[ends[0], ends[3]]);
                assert_eq!(a.add_stop(l, w), 0, "{what}: {}", a.issues());
                assert_one_stop(&a, l, e, e, w, &what);
            }
        }
    }
}

/// The partner a line's way back stops at cannot be taken away under it; a leg made two-way leaves
/// the pair standing aside (the line runs, each half a stop as any), and made one-way again it holds.
#[test]
fn a_pair_under_a_line_holds_or_stands_aside() {
    let (mut a, [p, q, n, s]) = pair(true, 20.0);
    let (e, w) = east_half(&a, n, s);
    assert_eq!(a.pair_stations(n, s), 0, "{}", a.issues());
    let l = new_line(&mut a, &[p, e, q]);
    assert_ne!(a.remove_station(w), 0, "the way back's half is in use");
    assert!(a.w.net.is_station(w));
    let net = &a.w.net;
    let leg = (0..net.edge_count() as u32).find(|&k| net.edge_ok(k) && net.edge_tracks[k as usize] == 1 && net.node_ports[w as usize].ends().iter().any(|&c| c >> 1 == k)).unwrap();
    let way = net.edge_way(leg);
    assert_eq!(a.set_edge_oneway(leg, 3), 0, "{}", a.issues());
    assert_eq!(a.station_pair(e), -1, "no pair on a two-way leg");
    assert!(a.w.net.check_line(l).is_ok(), "{:?}", a.w.net.check_line(l));
    assert_eq!(a.set_edge_oneway(leg, way), 0, "{}", a.issues());
    assert_eq!(a.station_pair(e), w as i32, "the pair holds again");
    assert_one_stop(&a, l, e, e, w, "one-way again");
}

/// A save keeps the pairs as set, and the line on them loads as it ran; a network without pairs
/// writes no block.
#[test]
fn pairs_round_trip_a_save() {
    let (mut a, [p, q, n, s]) = pair(false, 20.0);
    let (e, w) = east_half(&a, n, s);
    assert_eq!(a.pair_stations(n, s), 0, "{}", a.issues());
    let l = new_line(&mut a, &[p, e, q]);
    let bytes = crate::track::save::encode(&a.w.net);
    let mut b = TrackApi::new(-73.985, 40.758, false);
    assert!(b.load(&bytes));
    // a load numbers the nodes densely: each station found again where it stands
    let (old, net) = (&a.w.net, &b.w.net);
    let at = |k: u32| (0..net.node_count() as u32).find(|&m| net.node_ok(m) && (net.node_x[m as usize] - old.node_x[k as usize]).hypot(net.node_y[m as usize] - old.node_y[k as usize]) < 0.01).unwrap();
    let [p2, q2, e2, w2] = [p, q, e, w].map(at);
    assert_eq!((b.station_pair(e2), b.station_pair(w2)), (w2 as i32, e2 as i32));
    let l2 = (0..b.w.net.line_count() as u32).find(|&k| b.w.net.line_ok(k)).unwrap();
    assert_eq!(b.w.net.line_stops(l2), &[p2, e2, q2][..]);
    assert_one_stop(&b, l2, e2, e2, w2, "loaded");
    assert_eq!(a.w.net.line_stops(l).len(), 3);
    let (c, _) = pair(false, 20.0);
    assert_eq!(crate::track::save::encode(&c.w.net).windows(4).filter(|w| *w == b"LPA1").count(), 0, "no block without pairs");
}

/// Stops given whole (`set_stops`, `order_stops`) with the half run 0 does not stop at: the line lists
/// the other, as a click would, rather than turn back to stop at the one given.
#[test]
fn stops_given_with_the_other_half() {
    for right_hand in [true, false] {
        let (mut a, [p, q, n, s]) = pair(right_hand, 20.0);
        let (e, w) = east_half(&a, n, s);
        assert_eq!(a.pair_stations(n, s), 0, "{}", a.issues());
        for by_hand in [false, true] {
            let what = format!("right hand {right_hand}, by hand {by_hand}");
            let l = new_line(&mut a, &[p, q]);
            let r = if by_hand { a.order_stops(l, &[p, w, q]) } else { a.set_stops(l, &[p, w, q]) };
            assert_eq!(r, 0, "{what}: {}", a.issues());
            assert_eq!(a.w.net.line_stops(l), &[p, e, q][..], "{what}");
            assert_one_stop(&a, l, e, e, w, &what);
        }
    }
}

/// Constructed stations pair and unpair free, and the pair is undone as a blueprint's is: a pair is no
/// construction, so it leaves the undo history as it was.
#[test]
fn constructed_stations_pair_free_and_undoably() {
    let (mut a, [p, q, n, s]) = pair(true, 20.0);
    let (e, w) = east_half(&a, n, s);
    let l = new_line(&mut a, &[p, e, q]);
    assert_eq!(a.construct_all(), 0, "{}", a.issue_kinds());
    assert!([p, q, n, s].iter().all(|&k| a.w.net.station_data(k).is_some_and(|d| d.built)));
    let cash = a.cash();
    assert_eq!(a.pair_stations(n, s), 0, "{}", a.issues());
    assert!((a.cash() - cash).abs() < 1e-9, "free, {cash} to {}", a.cash());
    assert!(a.can_undo(), "undoable");
    assert_one_stop(&a, l, e, e, w, "constructed");
    assert_eq!(a.undo(), 0, "{}", a.issues());
    assert_eq!(a.station_pair(e), -1);
    assert_eq!(a.pair_stations(n, s), 0, "{}", a.issues());
    assert_eq!(a.unpair_station(n), 0, "{}", a.issues());
    assert!((a.cash() - cash).abs() < 1e-9, "unpaired free, {cash} to {}", a.cash());
    assert_eq!(a.w.net.line_stops(l).len(), 4, "both halves again");
}
