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
