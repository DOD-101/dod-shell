//! Items relating to the search results
//!
//! See: [`list`]

mod entry;
mod list;

pub use {
    entry::{ResultCategory, ResultEntry},
    list::{ResultList, ResultListInput},
};
