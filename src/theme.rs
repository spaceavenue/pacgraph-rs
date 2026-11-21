#[derive(Clone, Debug)]
pub struct Theme {
  pub bg: String,
  pub top: String,
  pub dep: String,
  pub link: String,
  pub hl_target: String,
  pub hl_dep: String,
  pub hl_req: String,
  pub stat: String,
}

impl Theme {
  pub fn catppuccin() -> Self {
    let s = |c: &str| c.to_string();
    Self {
      bg: s("#1e1e2e"),
      top: s("#89b4fa"),
      dep: s("#a6adc8"),
      link: s("#45475a"),
      hl_target: s("#f38ba8"),
      hl_dep: s("#a6e3a1"),
      hl_req: s("#cba6f7"),
      stat: s("#cdd6f4"),
    }
  }
}

impl Default for Theme {
  fn default() -> Self {
    Self::catppuccin()
  }
}
