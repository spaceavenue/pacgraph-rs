use std::fmt::Write;

use petgraph::Direction::{Incoming, Outgoing};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

use crate::compress::LayoutGraph;
use crate::highlight::Highlight;
use crate::layout::Layout;
use crate::pacman::DepKind;
use crate::theme::Theme;

pub fn human_size(bytes: u64) -> String {
  const UNITS: [&str; 3] = ["B", "KiB", "MiB"];
  let mut val = bytes as f64;
  let mut unit = 0;
  while val >= 1024.0 && unit < UNITS.len() - 1 {
    val /= 1024.0;
    unit += 1;
  }
  if unit == 0 {
    format!("{bytes} B")
  } else {
    format!("{val:.1} {}", UNITS[unit])
  }
}

fn escape(src: &str) -> String {
  src
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

/// Theme as CSS.
pub fn stylesheet(theme: &Theme) -> String {
  let Theme {
    bg,
    top,
    dep,
    link,
    hl_target,
    hl_dep,
    hl_req,
    stat,
  } = theme;
  format!(
    "<style>\n\
     rect.bg {{ fill: {bg}; }}\n\
     path.link {{ stroke: {link}; stroke-opacity: 0.2; fill: none; }}\n\
     path.opt {{ stroke-dasharray: 4 4; stroke-opacity: 0.12; }}\n\
     text.node {{ font-family: monospace, sans-serif; cursor: pointer; transition: opacity 0.15s; }}\n\
     text.node:hover {{ opacity: 0.65; }}\n\
     text.top {{ fill: {top}; }}\n\
     text.dep {{ fill: {dep}; }}\n\
     text.hl-target {{ fill: {hl_target}; font-weight: bold; }}\n\
     text.hl-dep {{ fill: {hl_dep}; }}\n\
     text.hl-req {{ fill: {hl_req}; }}\n\
     text.stat {{ fill: {stat}; font-family: monospace, sans-serif; font-weight: bold; }}\n\
     </style>",
  )
}

/// Control point of the quadratic Bézier drawn between two node positions.
/// `core` is the galaxy's centre, in the same coordinates as `from` and `to`.
fn control_point(from: (f64, f64), to: (f64, f64), core: (f64, f64)) -> (f64, f64) {
  let mid = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
  let (vx, vy) = (mid.0 - core.0, mid.1 - core.1);
  let chord = (to.0 - from.0).hypot(to.1 - from.1);
  let radius = vx.hypot(vy).max(1.0);
  let delta = (0.3 * chord / radius).min(0.6);
  let (sin, cos) = delta.sin_cos();
  (core.0 + vx * cos - vy * sin, core.1 + vx * sin + vy * cos)
}

/// Recolor the `svg` using `theme`.
pub fn recolor(svg: &str, theme: &Theme) -> Option<String> {
  let start = svg.find("<style>")?;
  let end = start + svg[start..].find("</style>")? + "</style>".len();
  Some(format!(
    "{}{}{}",
    &svg[..start],
    stylesheet(theme),
    &svg[end..]
  ))
}

/// Renders the svg.
pub fn render_svg(
  graph: &LayoutGraph,
  layout: &Layout,
  theme: &Theme,
  highlight: &Highlight,
) -> anyhow::Result<String> {
  let mut out = String::new();
  let (w, h) = (layout.width, layout.height);

  writeln!(
    out,
    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.0} {h:.0}" width="100%" height="100%">"#
  )?;
  writeln!(out, "{}", stylesheet(theme))?;
  writeln!(
    out,
    r#"<rect class="bg" x="0" y="0" width="{w:.0}" height="{h:.0}"/>"#
  )?;

  // edges
  writeln!(out, "<g>")?;
  for edge in graph.edge_references() {
    let (Some(a), Some(b)) = (
      layout.positions.get(&edge.source()),
      layout.positions.get(&edge.target()),
    ) else {
      continue;
    };
    let c = control_point(a.center, b.center, layout.origin);
    let class = match edge.weight() {
      DepKind::Required => "link",
      DepKind::Optional => "link opt",
    };
    writeln!(
      out,
      r#"<path class="{class}" d="M{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1}"/>"#,
      a.center.0, a.center.1, c.0, c.1, b.center.0, b.center.1
    )?;
  }
  writeln!(out, "</g>")?;

  // labels
  writeln!(out, "<g>")?;
  for idx in graph.node_indices() {
    let (node, Some(p)) = (&graph[idx], layout.positions.get(&idx)) else {
      continue;
    };
    let is_top = graph.edges_directed(idx, Incoming).next().is_none();
    let class = highlight.role_of(idx, is_top).class();

    let mut deps: Vec<&str> = graph
      .edges_directed(idx, Outgoing)
      .map(|e| graph[e.target()].name.as_str())
      .collect();
    deps.sort_unstable();
    let mut deps = if deps.is_empty() {
      "none".to_string()
    } else {
      deps.join(", ")
    };
    if deps.chars().count() > 80 {
      deps = deps.chars().take(77).collect::<String>() + "...";
    }

    let mut tip = format!("{}\nSize: {}", node.name, human_size(node.size));
    if node.merged_count > 1 {
      write!(tip, " ({} packages aggregated)", node.merged_count)?;
    }
    let kind = if node.explicit {
      "Explicitly installed"
    } else {
      "Dependency"
    };
    write!(tip, "\nType: {kind}\nDepends on: {deps}")?;

    // the collision rect is centred on the label, so its left edge is where text starts
    let x = p.rect.x1;
    writeln!(
      out,
      r#"<text x="{x:.1}" y="{:.1}" font-size="{:.0}" class="{class}"><title>{}</title>{}</text>"#,
      p.center.1,
      p.font_pt,
      escape(&tip),
      escape(&node.name),
    )?;
  }
  writeln!(out, "</g>")?;

  let total: u64 = graph.node_weights().map(|n| n.size).sum();
  writeln!(
    out,
    r#"<text x="40" y="55" font-size="28" class="stat">{} ({} nodes)</text>"#,
    human_size(total),
    graph.node_count()
  )?;
  writeln!(out, "</svg>")?;
  Ok(out)
}
