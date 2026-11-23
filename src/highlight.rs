use std::collections::{HashMap, HashSet};

use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{Bfs, Reversed};

use crate::compress::LayoutGraph;

/// What a node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
  /// Something the user asked to highlight.
  Target,
  /// Something a target needs, directly or transitively.
  Needs,
  /// Something that needs a target, directly or transitively.
  NeededBy,
  /// Nothing depends on it.
  Top,
  Dep,
}

impl Role {
  /// CSS classes for the label.
  pub fn class(self) -> &'static str {
    match self {
      Role::Target => "node hl-target",
      Role::Needs => "node hl-dep",
      Role::NeededBy => "node hl-req",
      Role::Top => "node top",
      Role::Dep => "node dep",
    }
  }
}

#[derive(Debug, Default)]
pub struct Highlight {
  pub targets: HashSet<NodeIndex>,
  /// Things reachable from a target by following dependencies.
  pub needs: HashSet<NodeIndex>,
  /// Things reaching a target by following dependencies.
  pub needed_by: HashSet<NodeIndex>,
}

impl Highlight {
  /// Resolves package names to nodes and walks the graph in both directions.
  pub fn new(graph: &LayoutGraph, names: &[String]) -> (Self, Vec<String>) {
    let by_member = graph
      .node_indices()
      .flat_map(|idx| graph[idx].members.iter().map(move |m| (m.as_str(), idx)))
      .collect::<HashMap<&str, NodeIndex>>();

    let mut hl = Self::default();
    let mut unknown = Vec::new();
    for name in names {
      match by_member.get(name.as_str()) {
        Some(&idx) => {
          hl.targets.insert(idx);
        }
        None => unknown.push(name.clone()),
      }
    }

    for &target in &hl.targets {
      let mut bfs = Bfs::new(graph, target);
      while let Some(n) = bfs.next(graph) {
        hl.needs.insert(n);
      }
      let reversed = Reversed(graph);
      let mut bfs = Bfs::new(&reversed, target);
      while let Some(n) = bfs.next(&reversed) {
        hl.needed_by.insert(n);
      }
    }
    hl.needs.retain(|n| !hl.targets.contains(n));
    hl.needed_by.retain(|n| !hl.targets.contains(n));
    (hl, unknown)
  }

  /// Decides which colour role a node gets.
  pub fn role_of(&self, idx: NodeIndex, is_top: bool) -> Role {
    if self.needs.contains(&idx) {
      Role::Needs
    } else if self.needed_by.contains(&idx) {
      Role::NeededBy
    } else if self.targets.contains(&idx) {
      Role::Target
    } else if is_top {
      Role::Top
    } else {
      Role::Dep
    }
  }
}
