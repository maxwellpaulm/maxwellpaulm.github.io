//! Deterministic tidy-tree layout for the automaton's trie structure.
//! Unit-free coordinates: y = depth, leaves at consecutive integer x in
//! trie insertion order, internal nodes centred over their children.

use crate::automaton::Automaton;

/// Post-order visit: leaves consume the next free integer x slot
/// left-to-right (children visited in insertion order); internal nodes
/// take the mean x of their children. Returns the x assigned to `s`.
fn visit(
    s: usize,
    a: &Automaton,
    children: &[Vec<usize>],
    next_x: &mut f32,
    pos: &mut [(f32, f32)],
) -> f32 {
    let y = a.depth(s) as f32;
    let x = if children[s].is_empty() {
        let x = *next_x;
        *next_x += 1.0;
        x
    } else {
        let sum: f32 = children[s].iter().map(|&c| visit(c, a, children, next_x, pos)).sum();
        sum / children[s].len() as f32
    };
    pos[s] = (x, y);
    x
}

/// Lays out every state of `a` in a unit-free coordinate space.
/// `Automaton` does not expose a children list, but state ids are
/// assigned in insertion order as the trie is built (`build` pushes a
/// new node the moment it is discovered and immediately records it as
/// its parent's child), so grouping states by `parent(s)` while walking
/// `s` in ascending order reconstructs each parent's children in the
/// same insertion order the trie built them in.
pub fn layout(a: &Automaton) -> Vec<(f32, f32)> {
    let n = a.node_count();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for s in 1..n {
        children[a.parent(s)].push(s);
    }
    let mut pos = vec![(0.0f32, 0.0f32); n];
    let mut next_x = 0.0f32;
    visit(0, a, &children, &mut next_x, &mut pos);
    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::Automaton;

    /// Walks the trie from the root along `word`, returning the state.
    /// Duplicated from `automaton.rs`'s test-only helper: it lives inside
    /// that module's `#[cfg(test)]` block and isn't importable across
    /// modules, and widening the production API just for test plumbing
    /// was already rejected in Task 1.
    fn state_for(a: &Automaton, word: &str) -> usize {
        let mut s = 0;
        for b in word.bytes() {
            s = (0..a.node_count())
                .find(|&n| a.parent(n) == s && a.label(n) == b)
                .expect("path exists");
        }
        s
    }

    #[test]
    fn ys_equal_depths_exactly() {
        let a = Automaton::build(&["he", "she", "his", "hers"]).unwrap();
        let pos = layout(&a);
        for (s, p) in pos.iter().enumerate() {
            assert_eq!(p.1, a.depth(s) as f32, "state {s}");
        }
    }

    #[test]
    fn a_lone_chain_is_a_vertical_line_at_x_zero() {
        let a = Automaton::build(&["abc"]).unwrap();
        let pos = layout(&a);
        assert_eq!(pos, vec![(0.0, 0.0), (0.0, 1.0), (0.0, 2.0), (0.0, 3.0)]);
    }

    #[test]
    fn two_disjoint_chains_split_and_root_centres() {
        // patterns "ab", "cd": leaves b at x=0, d at x=1 (insertion order),
        // a over b, c over d, root centred at 0.5.
        let a = Automaton::build(&["ab", "cd"]).unwrap();
        let pos = layout(&a);
        assert_eq!(pos[0].0, 0.5, "root centres over both subtrees");
        let xs: Vec<f32> = (1..5).map(|s| pos[s].0).collect();
        assert_eq!(xs, vec![0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn parents_centre_over_their_children() {
        let a = Automaton::build(&["he", "hi"]).unwrap();
        let pos = layout(&a);
        // h has children e (x=0) and i (x=1) → h at 0.5; root over h at 0.5.
        let h = (0..a.node_count()).find(|&s| a.label(s) == b'h').unwrap();
        assert_eq!(pos[h].0, 0.5);
        assert_eq!(pos[0].0, 0.5);
    }

    /// The textbook automaton's leaves sit at unequal depths (hers at 4,
    /// his and she at 3), so DFS leaf order (hers, his, she) differs from
    /// any level-order numbering (which would visit his/she before hers).
    /// Exact values pin the traversal order, not just the centring maths.
    #[test]
    fn asymmetric_depths_keep_dfs_leaf_order() {
        let a = Automaton::build(&["he", "she", "his", "hers"]).unwrap();
        let pos = layout(&a);
        assert_eq!(pos[state_for(&a, "hers")].0, 0.0);
        assert_eq!(pos[state_for(&a, "his")].0, 1.0);
        assert_eq!(pos[state_for(&a, "she")].0, 2.0);
        // h centres over e-subtree (0.0) and i-subtree (1.0); root over
        // h (0.5) and s (2.0).
        assert_eq!(pos[state_for(&a, "h")].0, 0.5);
        assert_eq!(pos[0].0, 1.25);
    }
}
