//! T-3376: a junction's merge lane, its price and the query the junction view reads.

use super::*;

fn api() -> TrackApi {
    let mut a = TrackApi::new(-73.985, 40.758, true);
    a.set_cash(1e9);
    a
}
fn free(x: f64, y: f64) -> [f64; 5] {
    [0.0, 0.0, x, y, 0.0]
}
fn ends(a: [f64; 5], b: [f64; 5]) -> Vec<f64> {
    a.iter().chain(b.iter()).copied().collect()
}
fn alive(a: &TrackApi) -> Vec<u32> {
    (0..a.w.net.edge_count() as u32).filter(|&e| a.w.net.edge_ok(e)).collect()
}
/// The node nearest (x, y) with 3+ edge ends.
fn junction_near(a: &TrackApi, x: f64, y: f64) -> u32 {
    let n = &a.w.net;
    (0..n.node_count() as u32)
        .filter(|&k| n.node_ok(k) && n.node_ports[k as usize].n >= 3)
        .min_by(|&p, &q| {
            let d = |k: u32| (n.node_x[k as usize] - x).hypot(n.node_y[k as usize] - y);
            d(p).total_cmp(&d(q))
        })
        .unwrap()
}
/// A main through (0, y)-(3000, y) and a branch leaving it at x = 1000 for (2000, y - 600).
fn y_junction(a: &mut TrackApi, y: f64) -> u32 {
    assert_eq!(a.add_route(&ends(free(0.0, y), free(3000.0, y)), &[], 2, false), 0, "{}", a.issues());
    let e = *alive(a).iter().find(|&&e| a.w.net.project(e, 1000.0, y).1 < 1.0).unwrap();
    assert_eq!(a.add_route(&ends([2.0, e as f64, 1000.0, y, 0.0], free(2000.0, y - 600.0)), &[], 2, false), 0, "{}", a.issues());
    junction_near(a, 1000.0, y)
}

#[test]
fn t3376_merge_lane_is_priced_toggled_and_saved() {
    let mut a = api();
    let j = y_junction(&mut a, 0.0);
    let before = a.merge_lane_info(j);
    assert_eq!(before.len(), 12);
    assert!((before[0] - 0.3 * 37.5).abs() < 1e-9, "0.3 km at ground: {}", before[0]);
    assert_eq!(before[1], 0.0, "not built yet");
    let quote = a.blueprint_cost();
    assert_eq!(a.set_merge_lane(j, true), 0, "{}", a.issues());
    assert!(a.w.net.node_merge_lane[j as usize]);
    assert!((a.blueprint_cost() - quote - before[0]).abs() < 1e-9, "the lane joins the quote");
    let on = a.merge_lane_info(j);
    assert_eq!((on[0], on[1]), (0.0, 1.0), "no price once it is there");
    // it rides in the save, and comes back off
    let saved = a.save();
    let mut b = api();
    assert!(b.load(&saved));
    assert!(b.w.net.node_merge_lane[j as usize]);
    assert_eq!(b.merge_lane_info(j)[1], 1.0);
    assert_eq!(a.set_merge_lane(j, false), 0);
    assert!(!a.w.net.node_merge_lane[j as usize]);
}
