/// Invalid models/options or execution limits.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An option violates its allowed range.
    #[error("invalid simulation options: {0}")]
    Options(&'static str),
    /// The tree is malformed or contains an operator unsupported by top-bottom.
    #[error("invalid tree: {0}")]
    Tree(String),
    /// A bound was reached; no partial log is returned.
    #[error("simulation exceeded {0}")]
    Limit(&'static str),
    /// The graph has no positive start weight or has a reachable dead end.
    #[error("invalid DFG: {0}")]
    Dfg(&'static str),
}
