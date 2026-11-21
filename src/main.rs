pub mod compress;
pub mod layout;
pub mod pacman;
pub mod render;
pub mod theme;

use layout::Layouter;

fn main() -> anyhow::Result<()> {
  let source = pacman::PacmanSource::default();
  let graph = source.build_graph(false)?;
  let layout_graph = compress::compress(&graph, compress::CompressOptions::default());
  println!(
    "{} packages -> {} nodes",
    graph.node_count(),
    layout_graph.node_count()
  );

  let placed = layout::galaxy::Galaxy::default().layout(&layout_graph);
  println!(
    "placed {} nodes in {:.0}x{:.0}px",
    placed.positions.len(),
    placed.width,
    placed.height
  );

  let svg = render::render_svg(&layout_graph, &placed, &theme::Theme::default())?;
  std::fs::write("pacgraph.svg", svg)?;
  println!("wrote pacgraph.svg");
  Ok(())
}
