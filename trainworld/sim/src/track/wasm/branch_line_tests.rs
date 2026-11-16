//! Line branches: the relation on a line and its `LBR1` save block.

use super::*;

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
