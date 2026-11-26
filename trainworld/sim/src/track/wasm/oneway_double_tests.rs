//! One-way double track (`Network::oneway_double`): double track the player set one way, both its
//! tracks carrying that way, a train keeping to the one the running side gives it (every other one drawn on the
//! other, `Trip::twin_track`).

use super::oneleg_tests::{api, at_node, end_station, ends, free, new_line, node_near, station_near};
use super::*;
use crate::track::capacity::{conflict_nodes, ResKind};
use crate::track::net::Network;
use crate::track::tests::{edge, line, node, station};

/// Half the distance between the legs: stations on them are within a walk of each other (`WAY_WALK_M`).
const GAP: f64 = 100.0;

/// Double track (0, 0) to J0 (2000, 0), two legs of `tracks` to J1 (4000, 0) through (3000, `GAP`)
/// and (3000, -`GAP`), then double to (6000, 0); both ways everywhere. Returns ([J0, J1], [north leg,
/// south leg]).
fn legs_track(right_hand: bool, tracks: u8) -> (TrackApi, [u32; 2], [u32; 2]) {
    let mut a = api(right_hand);
    assert_eq!(a.add_route(&ends(free(0.0, 0.0), free(2000.0, 0.0)), &[], 2, false), 0, "{}", a.issues());
    assert_eq!(a.add_route(&ends(free(4000.0, 0.0), free(6000.0, 0.0)), &[], 2, false), 0, "{}", a.issues());
    let (j0, j1) = (node_near(&a, 2000.0, 0.0), node_near(&a, 4000.0, 0.0));
    for y in [GAP, -GAP] {
        assert_eq!(a.add_route(&ends(at_node(&a, j0), at_node(&a, j1)), &[3000.0, y, 0.0, 0.0], tracks, false), 0, "{}", a.issues());
    }
    let net = &a.w.net;
    let joins = |e: u32| {
        let (x, y) = (net.edge_a[e as usize], net.edge_b[e as usize]);
        (x == j0 && y == j1) || (x == j1 && y == j0)
    };
    let leg = |y: f64| {
        let d = |e: u32| net.project(e, 3000.0, y).1;
        (0..net.edge_count() as u32).filter(|&e| net.edge_ok(e) && joins(e)).min_by(|&p, &q| d(p).total_cmp(&d(q))).unwrap()
    };
    let (north, south) = (leg(GAP), leg(-GAP));
    assert_ne!(north, south);
    (a, [j0, j1], [north, south])
}

/// The way that takes trains east (`east`) or west along piece `e`.
fn way(net: &Network, e: u32, east: bool) -> u8 {
    if (net.node_x[net.edge_a[e as usize] as usize] < net.node_x[net.edge_b[e as usize] as usize]) == east { 1 } else { 2 }
}

/// `legs_track` with the eastbound leg north (`east_north`) or south, each leg set one way, and end
/// stations P (0, 0) and Q (6000, 0). Returns ([P, Q, J0, J1], [eastbound leg, westbound leg]).
fn legs(right_hand: bool, tracks: u8, east_north: bool) -> (TrackApi, [u32; 4], [u32; 2]) {
    let (mut a, [j0, j1], [north, south]) = legs_track(right_hand, tracks);
    let (east, west) = if east_north { (north, south) } else { (south, north) };
    for (e, eastward) in [(east, true), (west, false)] {
        let w = way(&a.w.net, e, eastward);
        assert_eq!(a.set_edge_oneway(e, w), 0, "{}", a.issues());
        assert_eq!(a.edge_way(e), w);
    }
    let (p, q) = (end_station(&mut a, 0.0, 0.0), end_station(&mut a, 6000.0, 0.0));
    (a, [p, q, j0, j1], [east, west])
}

/// A line from P to Q, constructed with its track.
fn built_line(a: &mut TrackApi, p: u32, q: u32) -> u32 {
    let l = new_line(a, &[p, q]);
    assert_eq!(a.construct_line(l), 0, "{}", a.issues());
    assert!(a.w.svc.lines[l as usize].running);
    l
}

/// Two-way double track splitting into a one-way double leg each way, on both running sides: a line
/// runs out on one and back on the other, each train on the track the running side gives it, so the
/// legs cross the other way's track at the junctions (the flat-junction badge) only when the eastbound
/// leg leaves on the wrong side.
#[test]
fn one_way_double_legs_cross_only_on_the_wrong_side() {
    for right_hand in [true, false] {
        for east_north in [true, false] {
            let what = format!("right hand {right_hand}, eastbound leg north {east_north}");
            let (mut a, [p, q, j0, j1], [east, west]) = legs(right_hand, 2, east_north);
            let net = &a.w.net;
            for e in [east, west] {
                assert!(net.oneway_double(e), "{what}");
                let lanes = net.lanes(e);
                assert_eq!(lanes.all()[0].ways, lanes.all()[1].ways, "{what}: both tracks the one way");
            }
            let l = built_line(&mut a, p, q);
            let net = &a.w.net;
            let ls = &a.w.svc.lines[l as usize];
            assert!(ls.ok && ls.running && !net.line_broken[l as usize], "{what}");
            let uses = |r: usize, e: u32| ls.runs[r].segs.iter().any(|&s| s >> 1 == e);
            assert!(uses(0, east) && !uses(0, west), "{what}: out on the eastbound leg");
            assert!(uses(1, west) && !uses(1, east), "{what}: back on the westbound");
            for r in 0..2 {
                assert!(ls.runs[r].segs.iter().all(|&s| net.dir_ok(s)), "{what}: run {r} with the way");
                assert!(net.path_reversals(&ls.runs[r].segs).is_empty(), "{what}: run {r} turns back");
            }
            let wrong = east_north == right_hand;
            let flat = conflict_nodes(net, &a.w.svc.lines);
            assert_eq!(flat.contains(&j0), wrong, "{what}: J0 in {flat:?}");
            assert_eq!(flat.contains(&j1), wrong, "{what}: J1 in {flat:?}");
        }
    }
}

/// A single-track run joined to one track of double track (T-3137's join) takes its way from the
/// double where the player set that one way, whichever of its tracks the join names; set back both
/// ways, the join's track says again.
#[test]
fn single_track_joined_to_one_way_double_runs_its_way() {
    for right_hand in [true, false] {
        let what = format!("right hand {right_hand}");
        let mut a = api(right_hand);
        let n = [node(&mut a.w, 0.0, 0.0, 0), node(&mut a.w, 2000.0, 0.0, 0), node(&mut a.w, 4000.0, 0.0, 0)];
        let dbl = edge(&mut a.w, n[0], n[1], &[], 0).unwrap();
        let one = edge(&mut a.w, n[1], n[2], &[], 0).unwrap();
        // single track on from J, its end there on the track that brings trains into J
        let mut d = a.w.net.edge_data(one).unwrap();
        d.tracks = 1;
        d.join = [if right_hand { 1 } else { 2 }, 0];
        a.w.apply(Op::Edge { id: one, data: Some(d) }).unwrap();
        assert_eq!(a.edge_way(one), 2, "{what}: towards J, as the join's track runs");
        // the double one way into J: trains carry on from J along the single
        assert_eq!(a.set_edge_oneway(dbl, 1), 0, "{what}: {}", a.issues());
        assert_eq!(a.edge_way(one), 1, "{what}: on from J");
        // out of J: the single brings them in
        assert_eq!(a.set_edge_oneway(dbl, 2), 0, "{what}: {}", a.issues());
        assert_eq!(a.edge_way(one), 2, "{what}: into J");
        assert_eq!(a.set_edge_oneway(dbl, 1), 0, "{what}: {}", a.issues());
        assert_eq!(a.set_edge_oneway(dbl, 0), 0, "{what}: {}", a.issues());
        assert_eq!(a.edge_oneway(dbl), 0, "{what}");
        assert_eq!(a.edge_way(one), 2, "{what}: both ways again, the join's track");
    }
}

/// A one-way circle line round a ring of double track: with the ring set one way, its way, each train
/// holds its section half as long as on the same ring both ways, the other track taking the next train.
#[test]
fn one_way_double_halves_the_section_a_train_holds() {
    let occ = |oneway: bool| {
        let mut a = api(true);
        // (`oneway_tests`' triangle: AB straight, BC and CA round the long way)
        let n = [node(&mut a.w, 0.0, 0.0, 0), node(&mut a.w, 3000.0, 0.0, 0), node(&mut a.w, 1500.0, 3000.0, 0)];
        let pis: [&[(f64, f64)]; 3] = [&[], &[(5000.0, 0.0), (5000.0, 3000.0)], &[(-2000.0, 3000.0), (-2000.0, 0.0)]];
        let e: Vec<u32> = (0..3).map(|k| edge(&mut a.w, n[k], n[(k + 1) % 3], pis[k], 0).unwrap()).collect();
        for &x in &n {
            station(&mut a.w, x, 200);
        }
        let l = line(&mut a.w, &[n[0], n[1], n[2], n[0]], [8.0, 4.0, 2.0]);
        let mut d = a.w.net.line_data(l).unwrap();
        d.one_way = true;
        a.w.apply(Op::Line { id: l, data: Some(d) }).unwrap();
        if oneway {
            for &x in &e {
                assert_eq!(a.set_edge_oneway(x, 1), 0, "{}", a.issues());
                assert!(a.w.net.oneway_double(x));
            }
        }
        assert!(a.w.svc.lines[l as usize].running);
        let cap = &a.w.svc.cap;
        let mut occ: Vec<f64> = (0..cap.res.len()).filter(|&r| cap.res[r].kind == ResKind::Section).flat_map(|r| cap.users_of(r).iter().map(|u| u.occ)).collect();
        occ.sort_by(f64::total_cmp);
        occ
    };
    let (both, one) = (occ(false), occ(true));
    assert!(!both.is_empty() && both.len() == one.len(), "{both:?} against {one:?}");
    for (b, o) in both.iter().zip(&one) {
        assert!(*b > 0.0 && (o / b - 0.5).abs() < 1e-9, "{o} against {b}");
    }
}

/// The incremental capacity pass against the global one (as `capacity::tests`).
fn cap_like_global(a: &TrackApi, what: &str) {
    let mut g = crate::track::service::Service::default();
    g.update_all(&a.w.net);
    assert_eq!(a.w.svc.cap.canonical(), g.cap.canonical(), "{what}: resources");
}

/// Double track splitting into two legs, the westbound one-way double, and a station halfway along the
/// eastbound leg: with the first half of that leg one-way double, then all of it, then the first half
/// again (the rest plain double the line takes one way), a line's trains east pass as many sections as
/// with none of it one way, one-way double adding no queue of its own at either end. (A section cut at
/// each end of it put trains on Leganés's pair, a one-way double station each way, in more queues.)
#[test]
fn one_way_double_adds_no_section_of_its_own() {
    for right_hand in [true, false] {
        let what = format!("right hand {right_hand}");
        let (mut a, _, [north, south]) = legs_track(right_hand, 2);
        let st = station_near(&mut a, 3000.0, GAP);
        let net = &a.w.net;
        let mut halves: Vec<u32> = net.node_ports[st as usize].ends().iter().map(|&c| c >> 1).collect();
        assert_eq!(halves.len(), 2, "{what}: the station splits the leg");
        assert!(!halves.contains(&north) || net.edge_ok(north), "{what}");
        halves.sort_by(|&p, &q| net.project(p, 2000.0, 0.0).1.total_cmp(&net.project(q, 2000.0, 0.0).1));
        let w = way(net, south, false);
        assert_eq!(a.set_edge_oneway(south, w), 0, "{what}: {}", a.issues());
        let (p, q) = (end_station(&mut a, 0.0, 0.0), end_station(&mut a, 6000.0, 0.0));
        let l = built_line(&mut a, p, q);
        let sections = |a: &TrackApi| {
            let cap = &a.w.svc.cap;
            (0..cap.res.len()).filter(|&r| cap.res[r].kind == ResKind::Section && cap.users_of(r).iter().any(|u| u.line == l && u.run == 0)).count()
        };
        let none = sections(&a);
        for (k, set) in [(0, true), (1, true), (1, false)] {
            let e = halves[k];
            let w = if set { way(&a.w.net, e, true) } else { 0 };
            assert_eq!(a.set_edge_oneway(e, w), 0, "{what}: {}", a.issues());
            a.construct_all();
            let ls = &a.w.svc.lines[l as usize];
            assert!(ls.running && ls.runs[0].segs.iter().any(|&p| p >> 1 == e) && ls.runs[1].segs.iter().any(|&p| p >> 1 == south), "{what}: out on the leg, back on the westbound");
            cap_like_global(&a, &format!("{what}, half {k} way {w}"));
            assert_eq!(sections(&a), none, "{what}, half {k} way {w}: as many sections as with none of it one way");
        }
    }
}

/// A station on one-way double track is a one-way station (a leg stop), its two platforms the one
/// way's, as on one-way single legs: one on each leg, each a stop of its own served the way its leg runs.
#[test]
fn stations_on_one_way_double_are_one_way_stations() {
    for right_hand in [true, false] {
        let what = format!("right hand {right_hand}");
        let (mut a, [p, q, ..], _) = legs(right_hand, 2, !right_hand);
        let (n, s) = (station_near(&mut a, 3000.0, GAP), station_near(&mut a, 3000.0, -GAP));
        let net = &a.w.net;
        for x in [n, s] {
            assert!(net.leg_heading(x).is_some(), "{what}: {x} a one-way station");
            assert!(net.oneway_double_station(x), "{what}: both halves of its leg one way still");
        }
        let l = new_line(&mut a, &[p, q]);
        for x in [n, s] {
            assert_eq!(a.add_stop(l, x), 0, "{what}: {}", a.issues());
        }
        let net = &a.w.net;
        assert!(!net.line_broken[l as usize] && net.check_line(l).is_ok(), "{what}: the line runs, {:?}", net.check_line(l));
        assert_eq!(net.line_stops(l).len(), 4, "{what}: both one-way stations stops of their own");
    }
}

/// One-track stations (T-3332) and one-way double track exclude each other: track through a one-track
/// station refuses a way, and a station on one-way double refuses one track.
#[test]
fn one_track_stations_and_one_way_double_refuse_each_other() {
    let (mut a, _, _) = legs_track(true, 2);
    let st = station_near(&mut a, 3000.0, GAP);
    let halves: Vec<u32> = a.w.net.node_ports[st as usize].ends().iter().map(|&c| c >> 1).collect();
    assert_eq!(halves.len(), 2);
    assert_eq!(a.set_station_tracks(st, true), 0, "{}", a.issues());
    let w = way(&a.w.net, halves[0], true);
    assert_eq!(a.set_edge_oneway(halves[0], w), 1);
    assert_eq!(a.issue_kinds(), "OneTrackHere");
    assert_eq!(a.edge_oneway(halves[0]), 0, "nothing changed");
    // on both tracks again it takes the way, and then refuses one track
    assert_eq!(a.set_station_tracks(st, false), 0, "{}", a.issues());
    let ways: Vec<u8> = halves.iter().map(|&h| way(&a.w.net, h, true)).collect();
    assert_eq!(a.set_edges_oneway(&halves, &ways), 0, "{}", a.issues());
    assert!(a.w.net.oneway_double_station(st));
    assert_eq!(a.set_station_tracks(st, true), 1);
    assert_eq!(a.issue_kinds(), "OneTrackHere");
}

/// A one-track station placed on one-way double track is refused, as one switched there is.
#[test]
fn a_one_track_station_is_not_placed_on_one_way_double() {
    let (mut a, _, [east, _]) = legs(true, 2, false);
    let net = &a.w.net;
    let s = net.edge_len[east as usize] / 2.0;
    let (x, y, _) = crate::track::geom::pos_at(net.edge_pieces(east), s);
    a.set_station_one(&[x, y + 5.0]);
    assert_eq!(a.add_station(2, east, x, y, 120, "S".into()), 1);
    assert_eq!(a.issue_kinds(), "OneTrackHere");
    a.set_station_one(&[]);
    assert_eq!(a.add_station(2, east, x, y, 120, "S".into()), 0, "{}", a.issues());
}

/// One-way double track made single (T-3321) keeps its way, as one-way single track; made double again
/// it is refused (T-3296: a piece set one way stays single).
#[test]
fn one_way_double_made_single_keeps_its_way() {
    let (mut a, _, [east, west]) = legs(true, 2, false);
    let (we, ww) = (a.edge_way(east), a.edge_way(west));
    assert_eq!(a.set_edges_tracks(&[east, west], 1), 0, "{}", a.issues());
    for (e, w) in [(east, we), (west, ww)] {
        assert_eq!(a.w.net.edge_tracks[e as usize], 1);
        assert_eq!((a.edge_oneway(e), a.edge_way(e)), (w, w), "leg {e}");
    }
    assert_eq!(a.set_edges_tracks(&[east], 2), 1);
    assert_eq!(a.w.net.edge_tracks[east as usize], 1);
}

/// A double branch drawn off one-way double track with ramps on gets no flyover (T-3092's legs are for a main
/// that runs both ways): every train on the main runs one way, so its move to the branch crosses no other's, and
/// a leg for the way back would run into the main against its way.
#[test]
fn no_flyover_off_one_way_double() {
    for side in [1.0, -1.0] {
        let mut a = api(true);
        assert_eq!(a.add_route(&ends(free(-3000.0, 0.0), free(5000.0, 0.0)), &[], 2, false), 0, "{}", a.issues());
        let net = &a.w.net;
        let main = (0..net.edge_count() as u32).find(|&e| net.edge_ok(e)).unwrap();
        let w = way(net, main, true);
        assert_eq!(a.set_edge_oneway(main, w), 0, "{}", a.issues());
        a.set_route_ramp(true);
        let r = a.add_route(&ends([2.0, main as f64, 1000.0, 0.0, 0.0], free(3500.0, side * 1500.0)), &[], 2, true);
        a.set_route_ramp(false);
        assert_eq!(r, 0, "side {side}: {}", a.issues());
        let net = &a.w.net;
        assert!((0..net.edge_count() as u32).filter(|&e| net.edge_ok(e)).all(|e| net.edge_tracks[e as usize] == 2), "side {side}: no one-way legs");
        assert!((0..net.node_count() as u32).filter(|&n| net.node_ok(n)).all(|n| net.node_unusable(n) == 0), "side {side}: no track trains cannot use");
        let j = node_near(&a, 1000.0, 0.0);
        assert_eq!(net.node_ports[j as usize].n, 3);
        assert!(!crate::track::capacity::junction_crosses(net, j), "side {side}: no flat junction");
    }
}

/// The junction panel's flyover (a flying junction, `set_flying`) takes away the flat junction of one-way double
/// legs leaving on the wrong sides, the line running on as before.
#[test]
fn a_flying_junction_clears_legs_on_the_wrong_sides() {
    for right_hand in [true, false] {
        let what = format!("right hand {right_hand}");
        let (mut a, [p, q, j0, j1], _) = legs(right_hand, 2, right_hand);
        let l = built_line(&mut a, p, q);
        let flat = conflict_nodes(&a.w.net, &a.w.svc.lines);
        assert!(flat.contains(&j0) && flat.contains(&j1), "{what}: {flat:?}");
        for j in [j0, j1] {
            assert_eq!(a.set_flying(j, true), 0, "{what}: {}", a.issues());
        }
        let flat = conflict_nodes(&a.w.net, &a.w.svc.lines);
        assert!(!flat.contains(&j0) && !flat.contains(&j1), "{what}: {flat:?}");
        let ls = &a.w.svc.lines[l as usize];
        assert!(ls.ok && ls.running && !a.w.net.line_broken[l as usize], "{what}");
    }
}

/// Turning the running side over (T-3297) leaves one-way double track's way, and the single track set one way
/// that joins it, as they were: its way is the player's, not its tracks' side.
#[test]
fn flipping_the_running_side_keeps_one_way_double_and_its_joined_single() {
    let mut a = api(true);
    let n = [node(&mut a.w, 0.0, 0.0, 0), node(&mut a.w, 2000.0, 0.0, 0), node(&mut a.w, 4000.0, 0.0, 0)];
    let dbl = edge(&mut a.w, n[0], n[1], &[], 0).unwrap();
    let one = edge(&mut a.w, n[1], n[2], &[], 0).unwrap();
    let mut d = a.w.net.edge_data(one).unwrap();
    d.tracks = 1;
    d.join = [2, 0];
    a.w.apply(Op::Edge { id: one, data: Some(d) }).unwrap();
    assert_eq!(a.set_edge_oneway(dbl, 1), 0, "{}", a.issues());
    assert_eq!(a.set_edge_oneway(one, 1), 0, "{}", a.issues());
    let _ = a.flip_right_hand();
    assert!(!a.w.net.right_hand);
    assert_eq!((a.edge_oneway(dbl), a.edge_oneway(one)), (1, 1), "neither turned round");
    assert_eq!((a.edge_way(dbl), a.edge_way(one)), (1, 1));
    assert!((0..a.w.net.node_count() as u32).filter(|&k| a.w.net.node_ok(k)).all(|k| a.w.net.node_unusable(k) == 0));
}

/// Quad track (T-255: two doubles side by side) with each pair set one way, opposite ways: a line through it runs out
/// on one pair and back on the other, and the pair the player did not set stays both ways until set.
#[test]
fn quad_track_with_a_pair_each_way() {
    for right_hand in [true, false] {
        let what = format!("right hand {right_hand}");
        let mut a = api(right_hand);
        assert_eq!(a.add_route(&ends(free(-2000.0, 0.0), free(0.0, 0.0)), &[], 2, false), 0, "{what}: {}", a.issues());
        let w0 = node_near(&a, 0.0, 0.0);
        assert_eq!(a.add_route(&ends(at_node(&a, w0), free(6000.0, 0.0)), &[], 4, false), 0, "{what}: {}", a.issues());
        let e0 = node_near(&a, 6000.0, 0.0);
        assert_eq!(a.add_route(&ends(at_node(&a, e0), free(8000.0, 0.0)), &[], 2, false), 0, "{what}: {}", a.issues());
        // the main pair from W0 to E0, and the express pair beside it (in two pieces, a joint at its middle)
        let net = &a.w.net;
        let between = |e: u32| [net.edge_a[e as usize], net.edge_b[e as usize]].iter().all(|&n| (-1.0..=6001.0).contains(&net.node_x[n as usize]));
        let quad: Vec<u32> = (0..net.edge_count() as u32).filter(|&e| net.edge_ok(e) && between(e)).collect();
        let main: Vec<u32> = quad.iter().copied().filter(|&e| net.project(e, 3000.0, 0.0).1 < 1.0).collect();
        let express: Vec<u32> = quad.iter().copied().filter(|e| !main.contains(e)).collect();
        assert!(main.len() == 1 && !express.is_empty(), "{what}: {main:?} {express:?}");
        let (p, q) = (end_station(&mut a, -2000.0, 0.0), end_station(&mut a, 8000.0, 0.0));
        assert_eq!(a.set_edge_oneway(main[0], way(&a.w.net, main[0], true)), 0, "{what}: {}", a.issues());
        assert!(a.w.net.oneway_double(main[0]) && express.iter().all(|&e| !a.w.net.oneway_double(e)), "{what}");
        let ways: Vec<u8> = express.iter().map(|&e| way(&a.w.net, e, false)).collect();
        assert_eq!(a.set_edges_oneway(&express, &ways), 0, "{what}: {}", a.issues());
        let pairs = [main[0], express[0]];
        let l = built_line(&mut a, p, q);
        let net = &a.w.net;
        let ls = &a.w.svc.lines[l as usize];
        assert!(ls.ok && !net.line_broken[l as usize], "{what}");
        let uses = |r: usize, e: u32| ls.runs[r].segs.iter().any(|&s| s >> 1 == e);
        assert!(uses(0, pairs[0]) && !uses(0, pairs[1]), "{what}: out on the eastbound pair");
        assert!(uses(1, pairs[1]) && !uses(1, pairs[0]), "{what}: back on the westbound");
        assert!((0..net.node_count() as u32).filter(|&k| net.node_ok(k)).all(|k| net.node_unusable(k) == 0), "{what}");
    }
}

/// The way on double track survives a save (save level 42).
#[test]
fn one_way_double_survives_a_save() {
    let (a, _, legs) = legs(true, 2, false);
    let bytes = a.save();
    let mut b = api(true);
    assert!(b.load(&bytes));
    assert_eq!(b.oneway_edges(), a.oneway_edges());
    for e in legs {
        assert!(b.w.net.oneway_double(e), "leg {e}");
    }
}

/// Trains one after another on a run take one-way double track's two tracks in turn
/// (`Trip::twin_track`), each trip the same track from any window; `trips_with_tracks` is `trips`
/// and then those, one per trip.
#[test]
fn trains_take_one_way_doubles_two_tracks_in_turn() {
    let (mut a, [p, q, ..], _) = legs(true, 2, true);
    let l = built_line(&mut a, p, q);
    let t0 = a.w.svc.account_start(l as usize, 0.0) + 8.0 * 3600.0;
    let trips = a.w.svc.trips(&a.w.net, t0, t0 + 3600.0);
    for r in 0..2u8 {
        let mut run: Vec<_> = trips.iter().filter(|t| t.run == r).collect();
        run.sort_by(|x, y| x.dep.total_cmp(&y.dep));
        assert!(run.len() >= 4, "run {r}: {} trips in the hour", run.len());
        for w in run.windows(2) {
            if w[0].level == w[1].level && w[0].hw == w[1].hw && (w[1].dep - w[0].dep - w[0].hw).abs() < 1e-6 {
                assert_ne!(w[0].twin_track(), w[1].twin_track(), "run {r}: departures {} and {} on one track", w[0].dep, w[1].dep);
            }
        }
    }
    let later = a.w.svc.trips(&a.w.net, t0 + 1800.0, t0 + 5400.0);
    let mut both = 0;
    for t in &later {
        if let Some(u) = trips.iter().find(|u| u.run == t.run && u.dep == t.dep) {
            assert_eq!(u.twin_track(), t.twin_track(), "departure {} from either window", t.dep);
            both += 1;
        }
    }
    assert!(both > 0);
    let epoch = (t0 / 3600.0).floor() * 3600.0;
    let (plain, with) = (a.trips(epoch), a.trips_with_tracks(epoch));
    let n = plain.len() / 4;
    assert_eq!(with.len(), n * 5);
    assert_eq!(with[..n * 4].iter().map(|x| x.to_bits()).collect::<Vec<_>>(), plain.iter().map(|x| x.to_bits()).collect::<Vec<_>>());
    assert!(with[n * 4..].iter().all(|&x| x == 0.0 || x == 1.0) && with[n * 4..].contains(&1.0) && with[n * 4..].contains(&0.0));
}
