pub mod galaxy;
pub mod spatial;

use std::collections::HashMap;

use petgraph::stable_graph::NodeIndex;
use spatial::Rect;

use crate::compress::LayoutGraph;

/// Where a node ended up.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
  pub center: (f64, f64),
  pub font_pt: f64,
  pub rect: Rect,
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
  pub positions: HashMap<NodeIndex, Placed>,
  pub width: f64,
  pub height: f64,
  pub origin: (f64, f64),
}

/// Package size to its label font size mapping scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum FontScale {
  /// Font height proportional to size. Label area grows with size².
  Linear,
  /// Font height proportional to √size. Label area is proportional to size.
  #[default]
  Sqrt,
}

impl FontScale {
  /// Maps a package size to a font size. Value is clamped between `max_pt` and `min_pt`.
  pub fn font_pt(self, size: u64, max_size: u64, (min_pt, max_pt): (f64, f64)) -> f64 {
    let ratio = size as f64 / max_size.max(1) as f64;
    let t = match self {
      FontScale::Linear => ratio,
      FontScale::Sqrt => ratio.sqrt(),
    };
    (max_pt * t).max(min_pt)
  }
}

/// A layout strategy.
pub trait Layouter {
  fn layout(&self, g: &LayoutGraph) -> Layout;
}

/// Approximate monospace label size in px for a font size in pt.
pub fn text_dim(name: &str, pt: f64) -> (f64, f64) {
  (name.chars().count() as f64 * pt * 0.65, pt * 0.85)
}
