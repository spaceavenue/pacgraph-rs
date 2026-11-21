use std::collections::HashMap;
use std::ops::RangeInclusive;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
  pub x1: f64,
  pub y1: f64,
  pub x2: f64,
  pub y2: f64,
}

impl Rect {
  /// Box around a text label whose baseline origin is `center`:
  /// `h` above the baseline, `h / 3` below it (room for descenders).
  pub fn around_baseline(center: (f64, f64), w: f64, h: f64) -> Self {
    let (cx, cy) = center;
    Self {
      x1: cx - w / 2.0,
      y1: cy - h,
      x2: cx + w / 2.0,
      y2: cy + h / 3.0,
    }
  }

  pub fn overlaps(&self, o: &Rect) -> bool {
    self.x2 > o.x1 && self.x1 < o.x2 && self.y2 > o.y1 && self.y1 < o.y2
  }

  pub fn shifted(&self, dx: f64, dy: f64) -> Self {
    Self {
      x1: self.x1 + dx,
      y1: self.y1 + dy,
      x2: self.x2 + dx,
      y2: self.y2 + dy,
    }
  }
}

/// Uniform bucket grid.
pub struct SpatialGrid {
  cell: f64,
  cells: HashMap<(i32, i32), Vec<Rect>>,
}

impl SpatialGrid {
  pub fn new(cell: f64) -> Self {
    Self {
      cell,
      cells: HashMap::new(),
    }
  }

  fn cell_range(&self, rect: &Rect) -> (RangeInclusive<i32>, RangeInclusive<i32>) {
    let c = |v: f64| (v / self.cell).floor() as i32;
    (c(rect.x1)..=c(rect.x2), c(rect.y1)..=c(rect.y2))
  }

  pub fn insert(&mut self, rect: Rect) {
    let (xs, ys) = self.cell_range(&rect);
    for cx in xs {
      for cy in ys.clone() {
        self.cells.entry((cx, cy)).or_default().push(rect);
      }
    }
  }

  pub fn collides(&self, rect: &Rect) -> bool {
    let (xs, ys) = self.cell_range(rect);
    for cx in xs {
      for cy in ys.clone() {
        if let Some(bucket) = self.cells.get(&(cx, cy))
          && bucket.iter().any(|b| rect.overlaps(b))
        {
          return true;
        }
      }
    }
    false
  }
}
