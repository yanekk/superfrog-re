//! The movement map's graph (DESIGN §2.2): the type T04 fills and T06/T08 read, and shortest paths
//! over it in frames.
//!
//! A node is a standable cell plus whether the wings are held. T00 rejected speed bands (D15, D18):
//! edges start stopped and the frame-level search carries momentum.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::facts::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Node {
    pub cell: Cell,
    /// The wings held (`frog+$90`): flight moves are open from this node.
    pub wings: bool,
}

/// A macro move from one node to another, measured by stepping the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
    /// Held joystick states (`Joy` bits) and for how many frames each.
    pub inputs: Vec<(u8, u16)>,
    pub frames: u32,
    /// `Facts::coins` indices the move touches on the way (D17).
    pub coins: Vec<u16>,
    /// `Facts::switches` indices (as bits) that must be pressed for the move.
    pub needs_switches: u32,
    pub needs_wings: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveMap {
    pub nodes: Vec<Node>,
    /// Sorted by `from`.
    pub edges: Vec<Edge>,
    /// Per node, its outgoing edges' indices in `edges`.
    pub out: Vec<Range<u32>>,
}

impl MoveMap {
    /// A map of `nodes` and `edges` in any order: edges sorted by `from` (stably, so equal maps
    /// built in the same order stay byte-identical) and `out` filled.
    pub fn new(nodes: Vec<Node>, mut edges: Vec<Edge>) -> MoveMap {
        edges.sort_by_key(|e| e.from);
        let mut out = Vec::with_capacity(nodes.len());
        let mut at = 0u32;
        for n in 0..nodes.len() as u32 {
            let start = at;
            while (at as usize) < edges.len() && edges[at as usize].from == n {
                at += 1;
            }
            out.push(start..at);
        }
        MoveMap { nodes, edges, out }
    }
}

/// Frames from `from` to every node (`u32::MAX`: unreachable), Dijkstra on edge frames, using only
/// edges whose switches are all in `switches` and, unless `wings`, that need no wings.
pub fn shortest(map: &MoveMap, from: u32, switches: u32, wings: bool) -> Vec<u32> {
    let mut d = vec![u32::MAX; map.nodes.len()];
    let Some(start) = d.get_mut(from as usize) else {
        return d;
    };
    *start = 0;
    let mut q = BinaryHeap::from([Reverse((0u32, from))]);
    while let Some(Reverse((here, n))) = q.pop() {
        if here > d[n as usize] {
            continue;
        }
        let Some(out) = map.out.get(n as usize) else {
            continue;
        };
        for e in &map.edges[out.start as usize..out.end as usize] {
            if e.needs_switches & !switches != 0 || (e.needs_wings && !wings) {
                continue;
            }
            let nd = here.saturating_add(e.frames);
            if nd < d[e.to as usize] {
                d[e.to as usize] = nd;
                q.push(Reverse((nd, e.to)));
            }
        }
    }
    d
}
