use std::collections::VecDeque;

use petgraph::Direction::{Incoming, Outgoing};
use petgraph::graph::DiGraph;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;

use crate::pacman::{DepKind, InstallReason, Package};

/// A node laid out in the final graph.
#[derive(Clone, Debug)]
pub struct LayoutNode {
  pub name: String,
  pub size: u64,
  pub explicit: bool,
  pub merged_count: usize,
}

impl From<&Package> for LayoutNode {
  fn from(pkg: &Package) -> Self {
    Self {
      name: pkg.name.clone(),
      size: pkg.size,
      explicit: pkg.reason == InstallReason::Explicit,
      merged_count: 1,
    }
  }
}

pub type LayoutGraph = StableDiGraph<LayoutNode, DepKind>;

#[derive(Clone, Copy, Debug, Default)]
pub struct CompressOptions {
  pub preserve_explicit: bool,
}

/// Merges linear single-dependent chains into their dependents.
pub fn compress(graph: &DiGraph<Package, DepKind>, opts: CompressOptions) -> LayoutGraph {
  let mut g = LayoutGraph::from(graph.map(|_, pkg| LayoutNode::from(pkg), |_, &kind| kind));
  let mut queue = g.node_indices().collect::<VecDeque<NodeIndex>>();

  while let Some(s) = queue.pop_front() {
    if !g.contains_node(s) || (opts.preserve_explicit && g[s].explicit) {
      continue;
    }
    let Some(parent) = merge_target(&g, s) else {
      continue;
    };

    let deps = g
      .edges_directed(s, Outgoing)
      .map(|e| (e.target(), *e.weight()))
      .collect::<Vec<(NodeIndex, DepKind)>>();

    let (size, count) = (g[s].size, g[s].merged_count);
    g[parent].size += size;
    g[parent].merged_count += count;

    // put s's dependencies onto the parent, keeping Required over Optional
    for &(dep, kind) in &deps {
      if dep == parent {
        continue;
      }
      match g.find_edge(parent, dep) {
        Some(e) => {
          if kind == DepKind::Required {
            g[e] = DepKind::Required;
          }
        }
        None => {
          g.add_edge(parent, dep, kind);
        }
      }
    }
    g.remove_node(s);

    // each dependency swapped `s` for `parent` as a dependent so its count may have dropped
    queue.extend(deps.iter().map(|&(dep, _)| dep));
  }

  g
}

/// Returns the node `s` should be merged into, or `None` to leave `s` alone.
fn merge_target(g: &LayoutGraph, s: NodeIndex) -> Option<NodeIndex> {
  let mut dependents = g.edges_directed(s, Incoming);
  match (dependents.next(), dependents.next()) {
    (Some(e), None) if *e.weight() == DepKind::Required => Some(e.source()),
    _ => None,
  }
}
