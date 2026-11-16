//! Line branches: a head's trunk stop edits carried onto its branches.

use super::*;

impl TrackApi {
    /// Line branches: `d` as line `id`'s new data with each branch's stops following the head's
    /// edit, all in one op so one undo takes the lot back.
    pub(super) fn with_branches(&self, id: u32, d: LineData) -> Op {
        let net = &self.w.net;
        let old = net.line_stops(id);
        let mut ops = vec![];
        for b in net.branches_of(id) {
            let Some(mut bd) = net.line_data(b) else { continue };
            // a branch the new stops give no route keeps what it had: the head's edit still goes
            let Some((stops, path)) = branch_follow(old, &d.stops, &bd.stops).and_then(|s| net.route(&s).map(|p| (s, p))) else { continue };
            (bd.stops, bd.path) = (stops, path);
            ops.push(Op::Line { id: b, data: Some(bd) });
        }
        let head = Op::Line { id, data: Some(d) };
        if ops.is_empty() { head } else { ops.insert(0, head); Op::Batch(ops) }
    }
}

/// Line branches: `branch`'s stops once its head's went from `old` to `new`, None when unchanged.
/// Only the trunk they share moves: a stop put in between two the branch also runs between goes
/// in there too, and a stop taken out goes unless the branch starts or ends there (its own end
/// is the player's).
pub(super) fn branch_follow(old: &[u32], new: &[u32], branch: &[u32]) -> Option<Vec<u32>> {
    let last = branch.len().saturating_sub(1);
    let mut out: Vec<u32> = branch.iter().enumerate().filter(|&(i, s)| i == 0 || i == last || new.contains(s) || !old.contains(s)).map(|x| *x.1).collect();
    for (i, &s) in new.iter().enumerate() {
        if old.contains(&s) || out.contains(&s) || i == 0 {
            continue;
        }
        // after a run of new stops, the next old one: a run goes in one by one, each after the last
        let (x, y) = (new[i - 1], new[i + 1..].iter().copied().find(|t| old.contains(t)));
        let Some(y) = y else { continue };
        for k in (1..out.len()).rev() {
            if (out[k - 1], out[k]) == (x, y) || (out[k - 1], out[k]) == (y, x) {
                out.insert(k, s);
            }
        }
    }
    (out != branch).then_some(out)
}
