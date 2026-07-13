//! The results of the search
//!
//! The main item of the modle is [LauncherResults].
//! It wraps a [relm4::prelude::FactoryVecDeque], which contains the actual results and additionally stores the
//! currently selected item's index

mod category;
mod entry;
mod list;

pub(crate) use {category::ResultCategory, entry::ResultEntry, list::ResultList};
