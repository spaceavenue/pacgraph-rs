use std::collections::HashMap;
use std::fs::{self, DirEntry};
use std::path::PathBuf;

use anyhow::Context;
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstallReason {
  Explicit,
  Dependency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepKind {
  Required,
  Optional,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
  pub name: String,
  pub version: String,
  pub size: u64,
  pub reason: InstallReason,
  pub groups: Vec<String>,
  pub depends: Vec<String>,
  pub optdepends: Vec<String>,
  pub provides: Vec<String>,
}

pub struct PacmanSource {
  db_path: PathBuf,
}

impl PacmanSource {
  pub fn new(db_path: impl Into<PathBuf>) -> Self {
    let db_path = db_path.into();
    Self { db_path }
  }

  pub fn load_packages(&self) -> anyhow::Result<Vec<Package>> {
    let entries = fs::read_dir(&self.db_path)?
      .flatten()
      .collect::<Vec<DirEntry>>();
    let mut packages = Vec::new();
    for entry in entries {
      if !entry.file_type()?.is_dir() {
        continue;
      }
      let desc_entry = entry.path().join("desc");
      if !desc_entry.is_file() {
        continue;
      }
      let raw = fs::read_to_string(&desc_entry)?;
      packages.push(Self::parse_desc(&raw).context(format!(
        "failed to parse desc file: {}",
        desc_entry.display()
      ))?);
    }
    packages.sort();
    Ok(packages)
  }

  pub fn build_graph(&self, opt_include: bool) -> anyhow::Result<DiGraph<Package, DepKind>> {
    let packages = self.load_packages()?;
    let mut graph = DiGraph::with_capacity(packages.len(), packages.len() * 3);
    let mut by_name = HashMap::<String, NodeIndex>::with_capacity(packages.len());
    let mut by_provide = HashMap::<String, Vec<NodeIndex>>::new();

    // pass 1: insert all nodes and index exact names incl. virtual provides
    for pkg in packages {
      let name = pkg.name.clone();
      let provides = pkg.provides.clone();
      let idx = graph.add_node(pkg);

      by_name.insert(name, idx);

      for prov in provides {
        let base = sanitize_version(&prov);
        if base != prov {
          by_provide.entry(base.to_string()).or_default().push(idx);
        }
        by_provide.entry(prov).or_default().push(idx);
      }
    }

    // pass 2: resolve dependencies into edge pairs
    let mut edges = Vec::<(NodeIndex, NodeIndex, DepKind)>::new();
    for src_idx in graph.node_indices() {
      let pkg = &graph[src_idx];

      for raw_dep in &pkg.depends {
        if let Some(dst_idx) = resolve_dep(&graph, &pkg.name, raw_dep, &by_name, &by_provide) {
          edges.push((src_idx, dst_idx, DepKind::Required));
        }
      }
      if opt_include {
        for opt_dep in &pkg.optdepends {
          if let Some(dst_idx) = resolve_dep(&graph, &pkg.name, opt_dep, &by_name, &by_provide) {
            edges.push((src_idx, dst_idx, DepKind::Optional));
          }
        }
      }
    }

    // pass 3: dedup
    for (src, dst, kind) in edges {
      if src == dst {
        continue;
      }
      match graph.find_edge(src, dst) {
        Some(ei) => {
          if matches!(kind, DepKind::Required) {
            graph[ei] = DepKind::Required;
          }
        }
        None => {
          graph.add_edge(src, dst, kind);
        }
      }
    }

    Ok(graph)
  }

  fn parse_desc(raw: &str) -> anyhow::Result<Package> {
    let mut name = String::new();
    let mut version = String::new();
    let mut size = 0;
    let mut reason = InstallReason::Explicit;
    let mut groups = Vec::new();
    let mut depends = Vec::new();
    let mut optdepends = Vec::new();
    let mut provides = Vec::new();

    let mut current_selection: Option<&str> = None;

    for line in raw.lines() {
      let trimmed = line.trim();
      if trimmed.is_empty() {
        continue;
      }
      if trimmed.starts_with('%') && trimmed.ends_with('%') && trimmed.len() > 2 {
        current_selection = trimmed.get(1..trimmed.len() - 1);
        continue;
      }

      match current_selection {
        Some("NAME") => name = trimmed.to_string(),
        Some("VERSION") => version = trimmed.to_string(),
        Some("SIZE") => size = trimmed.parse::<u64>()?,
        Some("REASON") => {
          if trimmed == "1" {
            reason = InstallReason::Dependency;
          }
        }
        Some("GROUPS") => groups.push(trimmed.to_string()),
        Some("DEPENDS") => depends.push(trimmed.to_string()),
        Some("OPTDEPENDS") => {
          let dep = trimmed.split_once(": ").map_or(trimmed, |(n, _)| n);
          optdepends.push(dep.to_string());
        }
        Some("PROVIDES") => provides.push(trimmed.to_string()),
        _ => {}
      }
    }

    Ok(Package {
      name,
      version,
      size,
      reason,
      groups,
      depends,
      optdepends,
      provides,
    })
  }
}

fn sanitize_version(entry: &str) -> &str {
  entry.split(['=', '>', '<']).next().unwrap_or(entry)
}

fn pick_provider(
  graph: &DiGraph<Package, DepKind>,
  requester: &str,
  providers: &[NodeIndex],
) -> Option<NodeIndex> {
  let is_lib32 = requester.starts_with("lib32-");
  providers
    .iter()
    .copied()
    .find(|&idx| graph[idx].name.starts_with("lib32-") == is_lib32)
    .or_else(|| providers.first().copied())
}

fn resolve_dep(
  graph: &DiGraph<Package, DepKind>,
  requester: &str,
  dep: &str,
  by_name: &HashMap<String, NodeIndex>,
  by_provide: &HashMap<String, Vec<NodeIndex>>,
) -> Option<NodeIndex> {
  let base_dep = sanitize_version(dep);

  // by name
  if let Some(&dst_idx) = by_name.get(base_dep) {
    return Some(dst_idx);
  }

  // by provides with exact version (like libfoo.so=1-2)
  if let Some(providers) = by_provide.get(dep)
    && let Some(dst_idx) = pick_provider(graph, requester, providers)
  {
    return Some(dst_idx);
  }

  // by provides with no version / virtual (like sh -> bash)
  if let Some(providers) = by_provide.get(base_dep) {
    return pick_provider(graph, requester, providers);
  }

  None
}

impl Default for PacmanSource {
  fn default() -> Self {
    if let Ok(pacman_conf) = fs::read_to_string("/etc/pacman.conf") {
      for line in pacman_conf
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| l.starts_with("DBPath"))
      {
        if let Some((_, rest)) = line.split_once('=') {
          let db_path = PathBuf::from(rest.trim()).join("local");
          return Self::new(db_path);
        }
      }
    }
    Self::new("/var/lib/pacman/local")
  }
}
