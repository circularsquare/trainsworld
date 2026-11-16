//! Line branches: the relation on a line and its `LBR1` save block.

use super::*;
use crate::track::tests::{edge, node, station, terminals_at_dead_ends, world};

/// Two lines over the same pair of stations, the second set to branch from the first.
fn two_lines(branch: bool) -> (TrackApi, u32, u32) {
    let mut a = TrackApi::new(-73.985, 40.758, true);
    let ends = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4000.0, 0.0, 0.0];
    assert_eq!(a.add_route(&ends, &[], 2, false), 0, "{}", a.issues());
    let stops: Vec<u32> = [500.0, 3500.0].iter().map(|&x| {
        let e = (0..a.w.net.edge_count() as u32).filter(|&e| a.w.net.edge_ok(e)).min_by(|&p, &q| a.w.net.project(p, x, 0.0).1.total_cmp(&a.w.net.project(q, x, 0.0).1)).unwrap();
        assert_eq!(a.add_station(2, e, x, 0.0, 200, format!("S{x}")), 0, "{}", a.issues());
        (0..a.w.net.node_count() as u32).filter(|&k| a.w.net.node_ok(k) && a.w.net.node_platform[k as usize] > 0)
            .min_by(|&p, &q| (a.w.net.node_x[p as usize] - x).abs().total_cmp(&(a.w.net.node_x[q as usize] - x).abs())).unwrap()
    }).collect();
    let ids: Vec<u32> = (0..2).map(|_| {
        let id = a.new_line_id();
        assert_eq!(a.set_line(id, &stops, 6.0, 4.0, 2.0, 30.0, 180.0, 0, 0xff0000, format!("L{id}")), 0, "{}", a.issues());
        id
    }).collect();
    if branch {
        let mut d = a.w.net.line_data(ids[1]).unwrap();
        d.branch = Some(ids[0]);
        assert_eq!(a.run(Op::Line { id: ids[1], data: Some(d) }), 0, "{}", a.issues());
    }
    (a, ids[0], ids[1])
}

#[test]
fn a_branch_survives_a_save_and_load() {
    let (a, head, branch) = two_lines(true);
    assert_eq!(a.w.net.branches_of(head), vec![branch]);
    let mut b = TrackApi::new(-73.985, 40.758, true);
    assert!(b.load(&a.save()));
    assert_eq!(b.line_branch_of(branch), head as i32);
    assert_eq!(b.line_branch_of(head), -1);
}

#[test]
fn a_save_without_branches_loads_none() {
    let (a, head, other) = two_lines(false);
    let bytes = a.save();
    assert!(!bytes.windows(4).any(|w| w == b"LBR1"));
    let mut b = TrackApi::new(-73.985, 40.758, true);
    assert!(b.load(&bytes));
    assert_eq!([b.line_branch_of(head), b.line_branch_of(other)], [-1, -1]);
}

/// What a branch takes from its head: the line's look and its trains' kind and timings.
type Identity = (String, String, u32, u8, bool, u16, f32, f32, u32, String, GroupLook);
fn identity(a: &TrackApi, l: u32) -> Identity {
    let d = a.w.net.line_data(l).unwrap();
    (d.name, d.symbol, d.colour, d.cars, d.fast, d.limit, d.dwell_s, d.turnaround_s, d.group, d.group_name, d.group_look)
}

#[test]
fn renaming_the_head_renames_its_branches() {
    let (mut a, head, branch) = two_lines(true);
    let was = identity(&a, branch);
    assert_eq!(a.set_line_look(head, "Red".into(), 0x00ff00), 0, "{}", a.issues());
    assert_eq!((a.w.net.line_name[branch as usize].as_str(), a.w.net.line_colour[branch as usize]), ("Red", 0x00ff00));
    assert_eq!(a.undo(), 0, "{}", a.issues());
    assert_eq!(identity(&a, branch), was, "one undo puts the branch back too");
    assert_eq!(a.w.net.line_name[head as usize], format!("L{head}"));
    assert_eq!(a.line_branch_of(branch), head as i32);
}

#[test]
fn every_identity_edit_on_the_head_reaches_its_branches() {
    let (mut a, head, branch) = two_lines(true);
    let stops = a.w.net.line_stops(head).to_vec();
    // a group of one dissolves, so the head joins one with another line
    let other = a.new_line_id();
    assert_eq!(a.set_line(other, &stops, 6.0, 4.0, 2.0, 30.0, 180.0, 0, 0xff0000, "Other".into()), 0, "{}", a.issues());
    let g = a.new_line_group();
    let edits: [(&str, Box<dyn Fn(&mut TrackApi) -> u32>); 8] = [
        ("badge", Box::new(move |a| a.set_line_badge(head, "Blue".into(), 0x0000ff, "B".into()))),
        ("cars", Box::new(move |a| a.set_line_cars(head, 3))),
        ("fast", Box::new(move |a| a.set_line_fast(head, true))),
        ("limit", Box::new(move |a| a.set_line_limit(head, 80))),
        ("timings", Box::new(move |a| a.set_line_with_trains(head, &stops, 6, 4, 2, 45.0, 240.0, 3, 0x0000ff, "Blue".into()))),
        ("group", Box::new(move |a| a.set_line_group(&[head, other], g))),
        ("group name", Box::new(move |a| a.set_line_group_name(g, "G".into()))),
        ("group look", Box::new(move |a| a.set_line_group_look(g, "X".into(), 0x123456))),
    ];
    for (what, edit) in edits {
        let was = identity(&a, head);
        assert_eq!(edit(&mut a), 0, "{what}: {}", a.issues());
        assert_ne!(identity(&a, head), was, "{what}: the edit changed the head");
        assert_eq!(identity(&a, branch), identity(&a, head), "{what}");
        assert_eq!(a.line_branch_of(branch), head as i32, "{what}");
    }
    // the head's group loses the branch with the head, leaving one line, which is no group
    assert_eq!(a.remove_line(head), 0, "{}", a.issues());
    assert_eq!(a.line_group(other), 0);
}

#[test]
fn deleting_the_head_deletes_its_branches() {
    let (mut a, head, branch) = two_lines(true);
    assert_eq!(a.remove_line(head), 0, "{}", a.issues());
    assert!(!a.w.net.line_ok(head) && !a.w.net.line_ok(branch));
    assert_eq!(a.undo(), 0, "{}", a.issues());
    assert!(a.w.net.line_ok(head) && a.w.net.line_ok(branch));
    assert_eq!(a.line_branch_of(branch), head as i32);
    // the freed id taken again is a new line, with no branches of the old one
    assert_eq!(a.redo(), 0, "{}", a.issues());
    let stops = a.w.net.line_stops(head).to_vec();
    assert_eq!(a.set_line(head, &stops, 6.0, 4.0, 2.0, 30.0, 180.0, 0, 0xff0000, "New".into()), 0, "{}", a.issues());
    assert!(a.w.net.branches_of(head).is_empty());
}

#[test]
fn deleting_a_branch_keeps_the_head() {
    let (mut a, head, branch) = two_lines(true);
    assert_eq!(a.remove_line(branch), 0, "{}", a.issues());
    assert!(a.w.net.line_ok(head) && !a.w.net.line_ok(branch));
    assert!(a.w.net.branches_of(head).is_empty());
}

#[test]
fn a_branch_keeps_its_own_trains() {
    let (mut a, head, branch) = two_lines(true);
    assert_eq!(a.set_train_counts(branch, 2, 2, 2), 0, "{}", a.issues());
    assert_eq!(a.set_train_counts(head, 6, 6, 6), 0, "{}", a.issues());
    assert_eq!(a.set_line_look(head, "Red".into(), 0x00ff00), 0, "{}", a.issues());
    assert_eq!(a.w.net.line_trains[branch as usize], Some([2, 2, 2]));
    assert_eq!(a.w.net.line_trains[head as usize], Some([6, 6, 6]));
}

#[test]
fn a_branch_no_head_could_hold_goes_in_the_edit_that_makes_it() {
    // its own head
    let (mut a, _, branch) = two_lines(false);
    let mut d = a.w.net.line_data(branch).unwrap();
    d.branch = Some(branch);
    assert_eq!(a.run(Op::Line { id: branch, data: Some(d) }), 0, "{}", a.issues());
    assert!(!a.w.net.line_ok(branch));
    // a head made a branch itself takes its branches with it
    let (mut a, head, branch) = two_lines(true);
    let stops = a.w.net.line_stops(head).to_vec();
    let top = a.new_line_id();
    assert_eq!(a.set_line(top, &stops, 6.0, 4.0, 2.0, 30.0, 180.0, 0, 0xff0000, "Top".into()), 0, "{}", a.issues());
    let mut d = a.w.net.line_data(head).unwrap();
    d.branch = Some(top);
    assert_eq!(a.run(Op::Line { id: head, data: Some(d) }), 0, "{}", a.issues());
    assert!(!a.w.net.line_ok(branch));
    assert_eq!(a.line_branch_of(head), top as i32);
    assert_eq!(identity(&a, head), identity(&a, top));
}

#[test]
fn a_save_with_a_branch_no_head_could_hold_loads_it_as_a_plain_line() {
    let (mut a, head, branch) = two_lines(true);
    // a head that is its own branch: both relations are bad
    a.w.net.line_branch[head as usize] = Some(head);
    let mut b = TrackApi::new(-73.985, 40.758, true);
    assert!(b.load(&a.save()));
    assert!(b.w.net.line_ok(head) && b.w.net.line_ok(branch), "the lines are kept");
    assert_eq!([b.line_branch_of(head), b.line_branch_of(branch)], [-1, -1]);
}

#[test]
fn a_branch_on_both_pairs_keeps_its_twin_in_step() {
    let mut a = TrackApi::new(-73.985, 40.758, true);
    a.set_route_tangent(true);
    let ends = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4000.0, 0.0, 0.0];
    assert_eq!(a.add_route(&ends, &[], 4, false), 0, "{}", a.issues());
    let near = |a: &TrackApi, x: f64, y: f64| (0..a.w.net.node_count() as u32).filter(|&k| a.w.net.node_ok(k) && a.w.net.node_platform[k as usize] > 0)
        .min_by(|&p, &q| { let d = |n: u32| (a.w.net.node_x[n as usize] - x).hypot(a.w.net.node_y[n as usize] - y); d(p).total_cmp(&d(q)) }).unwrap();
    let mut stops = vec![];
    for y in [0.0, QUAD_SPACING] {
        for x in [500.0, 3500.0] {
            let e = (0..a.w.net.edge_count() as u32).filter(|&e| a.w.net.edge_ok(e) && !a.w.net.edge_crossover[e as usize])
                .min_by(|&p, &q| a.w.net.project(p, x, y).1.total_cmp(&a.w.net.project(q, x, y).1)).unwrap();
            assert_eq!(a.add_station(2, e, x, y, 200, format!("S{x}")), 0, "{}", a.issues());
            if y == 0.0 { stops.push(near(&a, x, y)); }
        }
    }
    let ids: Vec<u32> = (0..2).map(|_| {
        let id = a.new_line_id();
        assert_eq!(a.set_line_with_trains(id, &stops, 6, 4, 2, 30.0, 180.0, 0, 0xff0000, format!("L{id}")), 0, "{}", a.issues());
        id
    }).collect();
    let (head, branch) = (ids[0], ids[1]);
    let mut d = a.w.net.line_data(branch).unwrap();
    d.branch = Some(head);
    assert_eq!(a.run(Op::Line { id: branch, data: Some(d) }), 0, "{}", a.issues());
    assert_eq!(a.set_line_pairs(branch, true), 0, "{}", a.issues());
    let twin = a.line_twin(branch);
    assert!(twin >= 0, "the branch runs on both pairs");
    let twin = twin as u32;
    assert_eq!(a.line_branch_of(twin), -1, "the twin follows its own line, not the head");
    assert_eq!(a.w.net.branches_of(head), vec![branch]);
    assert_eq!(a.set_line_cars(head, 3), 0, "{}", a.issues());
    assert_eq!(a.w.net.line_cars[twin as usize], 3, "the twin takes what the branch took in the same edit");
    assert_eq!(a.remove_line(head), 0, "{}", a.issues());
    assert!(!a.w.net.line_ok(branch) && !a.w.net.line_ok(twin), "the branch's twin goes with it");
}

#[test]
fn a_save_with_a_branch_of_a_dead_or_bus_line_loads_it_as_a_plain_line() {
    for bus in [false, true] {
        let (mut a, _, branch) = two_lines(true);
        let h = a.new_line_id();
        if bus {
            assert_eq!(a.set_bus_line(h, &[5000.0, 6100.0, 1.0, 5000.0, 100.0, 1.0], 480, 600, 1200, 30.0, 120.0, 0, "B".into()), 0, "{}", a.issues());
        } else {
            let stops = a.w.net.line_stops(branch).to_vec();
            assert_eq!(a.set_line(h, &stops, 6.0, 4.0, 2.0, 30.0, 180.0, 0, 0, "Gone".into()), 0, "{}", a.issues());
            assert_eq!(a.remove_line(h), 0, "{}", a.issues());
        }
        a.w.net.line_branch[branch as usize] = Some(h);
        let mut b = TrackApi::new(-73.985, 40.758, true);
        assert!(b.load(&a.save()));
        assert!(b.w.net.line_ok(branch), "bus {bus}: the line is kept");
        assert_eq!(b.line_branch_of(branch), -1, "bus {bus}");
    }
}

// ---- add_branch

fn api(w: TrackWorld) -> TrackApi {
    let mut a = TrackApi::new(-73.985, 40.758, true);
    a.w = w;
    a
}

/// Stations A - B - C - D along x, and F on a spur leaving the track past C: `[A, B, C, D, F]`.
fn open_layout() -> (TrackApi, [u32; 5]) {
    let mut w = world();
    let [a, b, c, j, d] = [0.0, 3000.0, 6000.0, 7000.0, 10000.0].map(|x| node(&mut w, x, 0.0, 0));
    let f = node(&mut w, 8400.0, 1500.0, 0);
    for (p, q) in [(a, b), (b, c), (c, j), (j, d)] {
        edge(&mut w, p, q, &[], 0).unwrap();
    }
    edge(&mut w, j, f, &[(7400.0, 0.0)], 0).unwrap();
    for n in [a, b, c, d, f] {
        station(&mut w, n, 200);
    }
    terminals_at_dead_ends(&mut w);
    (api(w), [a, b, c, d, f])
}

/// A ring of stations A - B - C - D round anticlockwise, and S on a spur leaving it past C:
/// `[A, B, C, D, S]`.
fn ring_layout() -> (TrackApi, [u32; 5]) {
    let mut w = world();
    let r = 3000.0;
    let at = |deg: f64, k: f64| (k * deg.to_radians().cos(), k * deg.to_radians().sin());
    let degs = [0.0, 90.0, 180.0, 225.0, 270.0, 360.0];
    let ns: Vec<u32> = degs[..5].iter().map(|&g| { let (x, y) = at(g, r); node(&mut w, x, y, 0) }).collect();
    for i in 0..5 {
        let half = (degs[i + 1] - degs[i]) / 2.0;
        edge(&mut w, ns[i], ns[(i + 1) % 5], &[at(degs[i] + half, r / half.to_radians().cos())], 0).unwrap();
    }
    // the spur leaves J along the ring and bends out of it
    let (jx, jy) = at(225.0, r);
    let (t, o) = (at(-45.0, 1.0), at(225.0, 1.0));
    let s = node(&mut w, jx + 1400.0 * t.0 + 1500.0 * o.0, jy + 1400.0 * t.1 + 1500.0 * o.1, 0);
    edge(&mut w, ns[3], s, &[(jx + 400.0 * t.0, jy + 400.0 * t.1)], 0).unwrap();
    let stations = [ns[0], ns[1], ns[2], ns[4], s];
    for n in stations {
        station(&mut w, n, 200);
    }
    terminals_at_dead_ends(&mut w);
    (api(w), stations)
}

fn new_line(a: &mut TrackApi, stops: &[u32]) -> u32 {
    let id = a.new_line_id();
    assert_eq!(a.set_line(id, stops, 6.0, 6.0, 6.0, 30.0, 180.0, 0, 0xff0000, "Head".into()), 0, "{}", a.issues());
    id
}

fn live_lines(a: &TrackApi) -> usize {
    (0..a.w.net.line_count() as u32).filter(|&l| a.w.net.line_ok(l)).count()
}

#[test]
fn a_branch_starts_as_the_trunk_up_to_its_stop() {
    let (mut a, [sa, sb, sc, sd, sf]) = open_layout();
    let head = new_line(&mut a, &[sa, sb, sc, sd]);
    let id = a.new_line_id();
    assert_eq!(a.add_branch(head, id, sc), 0, "{}", a.issues());
    assert_eq!(a.w.net.line_stops(id), &[sa, sb, sc]);
    assert_eq!(a.line_branch_of(id), head as i32);
    assert_eq!(identity(&a, id), identity(&a, head));
    assert_eq!(a.add_stop(id, sf), 0, "{}", a.issues());
    assert_eq!(a.w.net.line_stops(id), &[sa, sb, sc, sf]);
    assert_eq!(a.w.net.line_stops(head), &[sa, sb, sc, sd], "the head keeps its stops");
}

#[test]
fn a_circle_takes_a_spur() {
    let (mut a, [sa, sb, sc, sd, ss]) = ring_layout();
    let head = new_line(&mut a, &[sa, sb, sc, sd]);
    assert_eq!(a.set_stops(head, &[sa, sb, sc, sd, sa]), 0, "{}", a.issues());
    assert_eq!(a.line_loop(head), 1);
    let id = a.new_line_id();
    assert_eq!(a.add_branch(head, id, sc), 0, "{}", a.issues());
    assert_eq!(a.add_stop(id, ss), 0, "{}", a.issues());
    assert_eq!(a.w.net.line_stops(id), &[sa, sb, sc, ss]);
    assert_eq!(a.line_loop(head), 1);
    for l in [head, id] {
        assert!(a.w.net.line_ok(l) && a.w.svc.lines[l as usize].ok, "line {l}");
    }
    // from the loop's first stop the branch is that stop alone, never the whole lap
    let first = a.new_line_id();
    assert_eq!(a.add_branch(head, first, sa), 0, "{}", a.issues());
    assert_eq!(a.w.net.line_stops(first), &[sa]);
}

/// The times a line's first run leaves its stop `k`, in the hour from 08:00 of day 1.
fn leaves(a: &TrackApi, l: u32, k: usize) -> Vec<f64> {
    let t0 = 86400.0 + 8.0 * 3600.0;
    let mut out = vec![];
    a.w.svc.run_deps(&a.w.net, l as usize, 0, t0, t0 + 3600.0, &mut out);
    let run = &a.w.svc.lines[l as usize].runs[0];
    out.iter().map(|t| t.dep + run.prof[t.level as usize].dep[k]).collect()
}

#[test]
fn a_branch_and_its_head_take_turns_on_the_trunk() {
    let (mut a, [sa, sb, sc, sd, ss]) = ring_layout();
    let head = new_line(&mut a, &[sa, sb, sc, sd]);
    assert_eq!(a.set_stops(head, &[sa, sb, sc, sd, sa]), 0, "{}", a.issues());
    assert_eq!(a.set_auto_headway(head, 600, 600, 600), 0, "{}", a.issues());
    let id = a.new_line_id();
    assert_eq!(a.add_branch(head, id, sc), 0, "{}", a.issues());
    assert_eq!(a.add_stop(id, ss), 0, "{}", a.issues());
    for l in [head, id] {
        assert!((a.w.net.line_tph[l as usize][HIGH] - 6.0).abs() < 0.01, "line {l}: {:?}", a.w.net.line_tph[l as usize]);
    }
    // B is the second stop of both
    let (h, b) = (leaves(&a, head, 1), leaves(&a, id, 1));
    assert!(!h.is_empty() && !b.is_empty(), "{h:?} {b:?}");
    let gap = h.iter().flat_map(|x| b.iter().map(move |y| (x - y).abs())).fold(f64::INFINITY, f64::min);
    assert!(gap >= 120.0, "the two leave B {gap:.0} s apart: head {h:.0?} branch {b:.0?}");
}

#[test]
fn a_branch_cannot_branch() {
    let (mut a, [sa, sb, sc, sd, _]) = open_layout();
    let head = new_line(&mut a, &[sa, sb, sc, sd]);
    let id = a.new_line_id();
    assert_eq!(a.add_branch(head, id, sc), 0, "{}", a.issues());
    let n = live_lines(&a);
    let twig = a.new_line_id();
    assert_eq!(a.add_branch(id, twig, sb), 1);
    assert_eq!(a.issue_kinds(), "LinePath");
    assert!(!a.w.net.line_ok(twig) && live_lines(&a) == n);
}

#[test]
fn a_bus_line_cannot_branch() {
    let (mut a, [sa, ..]) = open_layout();
    a.set_cash(1e6);
    let bus = a.new_line_id();
    assert_eq!(a.set_bus_line(bus, &[0.0, 1500.0, 1.0, 0.0, 100.0, 1.0], 480, 600, 1200, 30.0, 120.0, 0, "B".into()), 0, "{}", a.issues());
    let n = live_lines(&a);
    let id = a.new_line_id();
    assert_eq!(a.add_branch(bus, id, sa), 1);
    assert_eq!(a.issue_kinds(), "LinePath");
    assert!(!a.w.net.line_ok(id) && live_lines(&a) == n);
}

#[test]
fn a_stop_not_on_the_head_is_refused() {
    let (mut a, [sa, sb, sc, sd, _]) = open_layout();
    let head = new_line(&mut a, &[sa, sb, sc]);
    let n = live_lines(&a);
    let id = a.new_line_id();
    assert_eq!(a.add_branch(head, id, sd), 1);
    assert_eq!(a.issue_kinds(), "LinePath");
    let gone = a.new_line_id();
    assert_eq!(a.add_branch(gone, id, sa), 1);
    assert_eq!(a.issue_kinds(), "NoSuchLine");
    assert!(!a.w.net.line_ok(id) && live_lines(&a) == n);
}

#[test]
fn adding_a_branch_is_one_undo() {
    let (mut a, [sa, sb, sc, sd, _]) = open_layout();
    let head = new_line(&mut a, &[sa, sb, sc, sd]);
    let id = a.new_line_id();
    assert_eq!(a.add_branch(head, id, sb), 0, "{}", a.issues());
    assert_eq!(a.undo(), 0, "{}", a.issues());
    assert!(!a.w.net.line_ok(id) && a.w.net.branches_of(head).is_empty());
    assert_eq!(a.w.net.line_stops(head), &[sa, sb, sc, sd]);
    assert_eq!(a.redo(), 0, "{}", a.issues());
    assert_eq!((a.w.net.line_stops(id), a.line_branch_of(id)), (&[sa, sb][..], head as i32));
}
