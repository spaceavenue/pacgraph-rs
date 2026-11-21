pub mod compress;
pub mod layout;
pub mod pacman;

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
  Ok(())
}
