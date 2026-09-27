//! Data display: trees and tables.

#[cfg(feature = "data-table")]
pub mod data_table;
#[cfg(any(feature = "data-table", feature = "tree"))]
mod ellipsis;
#[cfg(any(feature = "data-table", feature = "tree"))]
mod interaction;
#[cfg(any(feature = "data-table", feature = "tree"))]
mod rows;
#[cfg(feature = "tree")]
pub mod tree;

#[cfg(feature = "data-table")]
pub use data_table::{DataTable, data_table};
#[cfg(feature = "tree")]
pub use tree::{Tree, tree};
