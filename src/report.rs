use crate::compress::{LayoutGraph, LayoutNode};
use crate::render::human_size;

/// Prints the `n` largest nodes by installed size.
pub fn print_largest(graph: &LayoutGraph, n: usize) {
  let mut nodes = graph.node_weights().collect::<Vec<&LayoutNode>>();
  nodes.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
  for node in nodes.into_iter().take(n) {
    let extra = if node.merged_count > 1 {
      format!(" (+{} merged)", node.merged_count - 1)
    } else {
      String::new()
    };
    println!("{:>10}  {}{extra}", human_size(node.size), node.name);
  }
}
