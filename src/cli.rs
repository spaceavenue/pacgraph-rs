use std::path::PathBuf;

use clap::Parser;

use crate::layout::FontScale;
use crate::layout::galaxy::GalaxyParams;
use crate::theme::Theme;

#[derive(Parser)]
#[command(name = "pacgraph", version, about)]
pub struct Cli {
  /// Packages to highlight in the graph.
  pub packages: Vec<String>,

  /// Include optional dependencies.
  #[arg(short = 'o', long)]
  pub opt_deps: bool,

  /// Never merge explicitly installed packages into their dependents.
  #[arg(short = 'e', long)]
  pub explicits: bool,

  /// Disable merging of single-dependent chains.
  #[arg(short = 'n', long)]
  pub no_compression: bool,

  /// Output file (default: pacgraph.svg).
  #[arg(short = 'f', long)]
  pub file: Option<PathBuf>,

  // layout
  /// Smallest and largest font size in points.
  #[arg(long, num_args = 2, value_names = ["MIN", "MAX"])]
  pub point: Option<Vec<f64>>,

  /// How package size maps to font size.
  #[arg(long, value_enum)]
  pub font_scale: Option<FontScale>,

  /// Number of spiral arms.
  #[arg(long)]
  pub arms: Option<usize>,

  /// Spiral tightness. Higher is looser.
  #[arg(long)]
  pub pitch: Option<f64>,

  /// Angular spread of each arm, in radians.
  #[arg(long)]
  pub jitter: Option<f64>,

  /// Target fraction of the disc covered by labels (ignored with --radius).
  #[arg(long)]
  pub fill: Option<f64>,

  /// Outer radius in px for disc.
  #[arg(long)]
  pub radius: Option<f64>,

  // colors
  /// Background color.
  #[arg(long)]
  pub bg: Option<String>,
  /// Color of packages nothing depends on.
  #[arg(long)]
  pub top: Option<String>,
  /// Color of dependency packages.
  #[arg(long)]
  pub dep: Option<String>,
  /// Link color.
  #[arg(long)]
  pub link: Option<String>,
  /// Color of highlighted target packages.
  #[arg(long)]
  pub highlight: Option<String>,
  /// Color of the statistics label.
  #[arg(long)]
  pub stat: Option<String>,
}

impl Cli {
  pub fn galaxy_params(&self) -> GalaxyParams {
    let mut params = GalaxyParams::default();
    if let Some(pt) = &self.point {
      params.min_pt = pt[0];
      params.max_pt = pt[1];
    }
    if let Some(v) = self.font_scale {
      params.font_scale = v;
    }
    if let Some(v) = self.arms {
      params.arms = v.max(1);
    }
    if let Some(v) = self.pitch {
      params.pitch = v;
    }
    if let Some(v) = self.jitter {
      params.jitter = v;
    }
    if let Some(v) = self.fill {
      params.fill = v;
    }
    if let Some(v) = self.radius {
      params.max_radius = Some(v);
    }
    params
  }

  pub fn theme(&self) -> Theme {
    let mut theme = Theme::default();
    let overrides = [
      (&self.bg, &mut theme.bg),
      (&self.top, &mut theme.top),
      (&self.dep, &mut theme.dep),
      (&self.link, &mut theme.link),
      (&self.highlight, &mut theme.hl_target),
      (&self.stat, &mut theme.stat),
    ];
    for (value, slot) in overrides {
      if let Some(v) = value {
        slot.clone_from(v);
      }
    }
    theme
  }
}
