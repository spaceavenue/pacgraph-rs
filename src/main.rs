pub mod compress;
pub mod pacman;

fn main() -> anyhow::Result<()> {
  let source = pacman::PacmanSource::default();
  let graph = source.build_graph(false)?;
  let layout_graph = compress::compress(&graph, compress::CompressOptions::default());
  println!(
    "{} packages -> {} nodes",
    graph.node_count(),
    layout_graph.node_count()
  );
  Ok(())
}
