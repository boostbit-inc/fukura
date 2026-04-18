//! Built-in adapters shipped with fukura.
//!
//! Each adapter is a small, focused piece of logic that recognises a
//! specific family of errors. Adding a new adapter is the preferred way to
//! extend fukura's understanding of new tools or environments.

mod cargo;
mod generic;
mod git;
mod kubernetes;

pub use cargo::CargoAdapter;
pub use generic::GenericAdapter;
pub use git::GitAdapter;
pub use kubernetes::KubernetesAdapter;
