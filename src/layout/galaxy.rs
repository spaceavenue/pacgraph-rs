use std::collections::HashMap;
use std::f64::consts::{PI, TAU};

use petgraph::Direction::{Incoming, Outgoing};
use petgraph::stable_graph::NodeIndex;

use super::spatial::{Rect, SpatialGrid};
use super::{Layout, Layouter, Placed, text_dim};
use crate::compress::LayoutGraph;
use crate::layout::FontScale;

#[derive(Clone, Copy, Debug)]
pub struct GalaxyParams {
  pub arms: usize,
  pub pitch: f64,
  pub jitter: f64,
  pub core_radius: f64,
  pub max_radius: Option<f64>,
  pub fill: f64,
  pub min_pt: f64,
  pub max_pt: f64,
  pub font_scale: FontScale,
  pub padding: f64,
}

impl Default for GalaxyParams {
  fn default() -> Self {
    Self {
      arms: 3,
      pitch: 0.275,
      jitter: 0.6,
      core_radius: 20.0,
      max_radius: None,
      fill: 0.7,
      min_pt: 14.0,
      max_pt: 48.0,
      font_scale: FontScale::Sqrt,
      padding: 60.0,
    }
  }
}

#[derive(Default)]
pub struct Galaxy {
  pub params: GalaxyParams,
}

/// Per-node numbers.
pub struct NodeStats<'a> {
  pub name: &'a str,
  pub idx: NodeIndex,
  pub in_deg: usize,
  pub out_deg: usize,
  pub size: u64,
  pub merged_count: usize,
  pub explicit: bool,
}

/// Maxima over the whole graph.
pub struct Population {
  pub max_in_deg: usize,
  pub max_out_deg: usize,
  pub max_size: u64,
  pub max_merged_count: usize,
  pub rank: HashMap<NodeIndex, f64>,
}

impl Population {
  fn measure(graph: &LayoutGraph) -> Self {
    let mut by_degree = graph
      .node_indices()
      .map(|i| {
        (
          std::cmp::Reverse(graph.edges_directed(i, Incoming).count()),
          graph[i].name.as_str(),
          i,
        )
      })
      .collect::<Vec<_>>();
    by_degree.sort();

    let n = (by_degree.len().max(2) - 1) as f64;
    let rank = by_degree
      .iter()
      .enumerate()
      .map(|(r, &(_, _, i))| (i, r as f64 / n))
      .collect::<HashMap<NodeIndex, f64>>();

    let mut pop = Self {
      max_in_deg: 1,
      max_out_deg: 1,
      max_size: 1,
      max_merged_count: 1,
      rank,
    };
    for idx in graph.node_indices() {
      let stats = NodeStats::of(graph, idx);
      pop.max_in_deg = pop.max_in_deg.max(stats.in_deg);
      pop.max_out_deg = pop.max_out_deg.max(stats.out_deg);
      pop.max_size = pop.max_size.max(stats.size);
      pop.max_merged_count = pop.max_merged_count.max(stats.merged_count);
    }
    pop
  }
}

impl<'a> NodeStats<'a> {
  fn of(graph: &'a LayoutGraph, idx: NodeIndex) -> Self {
    let node = &graph[idx];
    Self {
      name: &node.name,
      idx,
      in_deg: graph.edges_directed(idx, Incoming).count(),
      out_deg: graph.edges_directed(idx, Outgoing).count(),
      size: node.size,
      merged_count: node.merged_count,
      explicit: node.explicit,
    }
  }
}

/// Deterministic pseudo-random value in 0..1 derived from a name (FNV-1a).
/// For identical jitter on every run.
#[allow(dead_code)]
pub fn hash01(name: &str) -> f64 {
  let mut thing: u64 = 0xcbf29ce484222325;
  for byte in name.bytes() {
    thing ^= u64::from(byte);
    thing = thing.wrapping_mul(0x100000001b3);
  }
  (thing >> 11) as f64 / (1u64 << 53) as f64
}

/// Calculates where this node should ideally sit. Returns `(radius, angle)` in px and radians.
fn ideal_polar(
  stats: &NodeStats,
  pop: &Population,
  params: &GalaxyParams,
  max_radius: f64,
) -> (f64, f64) {
  let (core_radius, pitch) = (params.core_radius, params.pitch);
  let radius = core_radius + (max_radius - core_radius) * pop.rank[&stats.idx].sqrt();
  let hash = hash01(stats.name) * params.arms as f64;
  let arm = hash.floor();
  let jitter = (hash - arm - 0.5) * params.jitter;
  let theta = (radius / core_radius).ln() / pitch + arm * TAU / params.arms as f64 + jitter;
  (radius, theta)
}

impl Layouter for Galaxy {
  fn layout(&self, graph: &LayoutGraph) -> Layout {
    if graph.node_count() == 0 {
      return Layout::default();
    }
    let params = &self.params;
    let pop = Population::measure(graph);

    // font sizes scale with package size relative to the largest. floored at min_pt
    let max_size = graph.node_weights().map(|n| n.size).max().unwrap_or(1);
    let fonts = graph
      .node_indices()
      .map(|idx| {
        let pt =
          params
            .font_scale
            .font_pt(graph[idx].size, max_size, (params.min_pt, params.max_pt));
        (idx, pt)
      })
      .collect::<HashMap<NodeIndex, f64>>();

    // size the disc to the labels it has to hold, unless a radius was provided
    let max_radius = params
      .max_radius
      .unwrap_or_else(|| {
        let area = graph
          .node_indices()
          .map(|idx| {
            let (w, h) = text_dim(&graph[idx].name, fonts[&idx]);
            w * h
          })
          .sum::<f64>();
        (area / (PI * params.fill)).sqrt()
      })
      .max(params.core_radius * 2.0);

    // nodes are placed in order of increasing radius, so small radii go first. if the ideal spot is
    // taken, slide the node outward along its arm.
    let mut order = graph
      .node_indices()
      .map(|idx| {
        let (r, theta) = ideal_polar(&NodeStats::of(graph, idx), &pop, params, max_radius);
        (idx, r, theta)
      })
      .collect::<Vec<(NodeIndex, f64, f64)>>();
    order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));

    let mut grid = SpatialGrid::new(100.0);
    let mut placed = HashMap::<NodeIndex, Placed>::with_capacity(order.len());

    for (idx, r0, theta0) in order {
      let node = &graph[idx];
      let pt = fonts[&idx];
      let (w, h) = text_dim(&node.name, pt);

      // slide outward and sweep around until the label fits
      let (mut r, mut theta) = (r0, theta0);
      let (center, rect) = loop {
        let center = (r * theta.cos(), r * theta.sin());
        let rect = Rect::around_baseline(center, w, h);
        if !grid.collides(&rect) {
          break (center, rect);
        }
        r += pt * 0.5;
        theta += 0.05;
      };
      grid.insert(rect);
      placed.insert(
        idx,
        Placed {
          center,
          font_pt: pt,
          rect,
        },
      );
    }

    // shift everything so the bounds start at (padding, padding)
    let min_x = placed.values().map(|n| n.rect.x1).fold(f64::MAX, f64::min);
    let min_y = placed.values().map(|n| n.rect.y1).fold(f64::MAX, f64::min);
    let (dx, dy) = (params.padding - min_x, params.padding - min_y);
    for n in placed.values_mut() {
      n.center = (n.center.0 + dx, n.center.1 + dy);
      n.rect = n.rect.shifted(dx, dy);
    }
    let width = placed.values().map(|n| n.rect.x2).fold(0.0, f64::max) + params.padding;
    let height = placed.values().map(|n| n.rect.y2).fold(0.0, f64::max) + params.padding;

    Layout {
      positions: placed,
      width,
      height,
      origin: (dx, dy),
    }
  }
}
