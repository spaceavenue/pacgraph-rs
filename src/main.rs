pub mod cli;
pub mod compress;
pub mod layout;
pub mod pacman;
pub mod render;
pub mod theme;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use cli::Cli;
use compress::CompressOptions;
use layout::Layouter;

fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();
  let theme = cli.theme();

  let source = pacman::PacmanSource::default();
  let graph = source.build_graph(cli.opt_deps)?;
  let layout_graph = compress::compress(
    &graph,
    CompressOptions {
      preserve_explicit: cli.explicits,
      skip: cli.no_compression,
    },
  );

  println!(
    "{} packages -> {} nodes",
    graph.node_count(),
    layout_graph.node_count()
  );
  let placed = layout::galaxy::Galaxy {
    params: cli.galaxy_params(),
  }
  .layout(&layout_graph);

  let mut out = cli
    .file
    .clone()
    .unwrap_or_else(|| PathBuf::from("pacgraph.svg"));
  if out
    .extension()
    .is_none_or(|e| !e.eq_ignore_ascii_case("svg"))
  {
    out.set_extension("svg");
  }
  let svg = render::render_svg(&layout_graph, &placed, &theme)?;
  std::fs::write(&out, svg).with_context(|| format!("failed to write {}", out.display()))?;
  println!(
    "wrote {} ({:.0}x{:.0}px)",
    out.display(),
    placed.width,
    placed.height
  );
  Ok(())
}
