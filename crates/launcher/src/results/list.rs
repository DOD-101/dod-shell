//! Module for the top-level [`ResultList`] struct
//!
//! It is responsible for holding all results, split into different [`ResultCategory`]s.
//!
//! ## Row
//!
//! A row in the context of this module is an index inside the results, ignoring their nesting
//! inside categories. Imagine an index into results, as if it were a flat list.
use relm4::prelude::*;

use crate::results::{ResultEntry, category::ResultCategoryInput, entry::ResultEntryInput};

use super::category::ResultCategory;

/// A two-level index pointing to a specific entry within a category of results.
///
/// This is the core addressing scheme for the results list: results are grouped into
/// categories, and each category contains an ordered list of entries. A [`ResultIndex`]
/// identifies exactly one entry by pairing a category index with an entry index.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResultIndex {
    /// Index of the category
    category: usize,
    /// Index of the entry in the category
    entry: usize,
}

impl ResultIndex {
    /// Creates a new [`ResultIndex`].
    const fn new(category: usize, entry: usize) -> Self {
        Self { category, entry }
    }
}

/// Pure navigation state, free of GTK dependencies.
///
/// Holds category lengths and the current selection index, and provides the navigation logic
/// for moving up and down through results.
#[derive(Debug)]
struct NavigationState {
    /// Number of entries in each category
    category_lengths: Vec<usize>,
    /// Index of the currently selected result entry
    selected: Option<ResultIndex>,
    /// The maximum amount of entries to show
    max_entries: usize,
}

impl NavigationState {
    /// Creates a new [`NavigationState`].
    const fn new(max_entries: usize) -> Self {
        Self {
            category_lengths: Vec::new(),
            selected: None,
            max_entries,
        }
    }

    /// Update category lengths and reset selection to the first entry
    fn set_results(&mut self, lengths: Vec<usize>) {
        self.category_lengths = lengths;
        self.selected = self.min_index();
    }

    /// Returns the selected index of this [`NavigationState`].
    const fn selected(&self) -> Option<&ResultIndex> {
        self.selected.as_ref()
    }

    /// Converts a [`ResultIndex`] to its flat row position.
    fn row_of(&self, index: &ResultIndex) -> usize {
        self.category_lengths[..index.category]
            .iter()
            .sum::<usize>()
            + index.entry
    }

    /// Go down one in the list of results (increasing the index)
    fn down(&mut self) {
        let Some(mut index) = self.selected.take() else {
            return;
        };

        if self.category_lengths[index.category] > index.entry + 1 {
            // Go down one result in the current category
            index.entry += 1;
        } else if index.category + 1 < self.category_lengths.len() {
            // Go to the first entry in the next category
            index.category += 1;
            index.entry = 0;
        } else {
            // If at the bottom, wrap back to the top
            index = self
                .min_index()
                .expect("There should be a valid index since we got a reference from self.");
        }

        // Respect max_entries: if the new position is beyond the limit, wrap to start
        if self.row_of(&index) >= self.max_entries {
            index = self
                .min_index()
                .expect("There should be a valid index since we got a reference from self.");
        }

        self.selected = Some(index);
    }

    /// Go up one in the list of results (decrease the index)
    fn up(&mut self) {
        let Some(mut index) = self.selected.take() else {
            return;
        };

        if index.entry > 0 {
            // Go up one result in the current category
            index.entry -= 1;
        } else if index.category > 0 {
            // Go to the last entry in the previous category
            index.category -= 1;
            index.entry = self.category_lengths[index.category] - 1;
        } else {
            // At the top, wrap back to the bottom
            index = self
                .max_index()
                .expect("There should be a valid index since we got a reference from self.");
        }

        self.selected = Some(index);
    }

    /// Gets the highest possible valid index.
    fn max_index(&self) -> Option<ResultIndex> {
        if self.is_empty() {
            return None;
        }

        let mut row = 0;

        for (i, &len) in self.category_lengths.iter().enumerate() {
            row += len;

            if row >= self.max_entries {
                let overshoot = row - self.max_entries;
                let entry = len - overshoot - 1;

                return Some(ResultIndex::new(i, entry));
            }
        }

        // Total entries < max_entries; all categories are non-empty
        let category = self.category_lengths.len() - 1;
        Some(ResultIndex::new(
            category,
            self.category_lengths[category] - 1,
        ))
    }

    /// Gets the lowest possible valid index.
    const fn min_index(&self) -> Option<ResultIndex> {
        if self.is_empty() {
            return None;
        }

        Some(ResultIndex::new(0, 0))
    }

    /// Returns if there are no entries.
    ///
    /// All categories are assumed to be non-empty, so emptiness means the vec itself is empty.
    const fn is_empty(&self) -> bool {
        self.category_lengths.is_empty()
    }
}

/// See module level documentation
#[derive(Debug)]
pub struct ResultList {
    /// Categories of results
    categories: FactoryVecDeque<ResultCategory>,
    /// Pure navigation state
    nav: NavigationState,
}

impl ResultList {
    /// Creates a new [`ResultList`].
    pub fn new(max_entries: usize) -> Self {
        Self {
            categories: FactoryVecDeque::builder().launch_default().detach(),
            nav: NavigationState::new(max_entries),
        }
    }

    /// Set [`Self::categories`] overriding any old values entirely
    ///
    /// Empty categories are dropped.
    pub fn set_results(&mut self, categories: Vec<ResultCategory>) {
        {
            let mut guard = self.categories.guard();

            guard.clear();

            let mut row = 0;

            for mut result in categories {
                if result.is_empty() {
                    continue;
                }

                row += result.len();
                if row > self.nav.max_entries {
                    let overflow = row - self.nav.max_entries;

                    {
                        let mut guard = result.entries.guard();

                        for _ in 0..overflow {
                            guard.pop_back();
                        }
                    }
                }
                guard.push_back(result);

                if row >= self.nav.max_entries {
                    break;
                }
            }
        }

        let lengths: Vec<usize> = (0..self.categories.len())
            .map(|i| self.categories[i].len())
            .collect();
        self.nav.set_results(lengths);
        self.set_current_result_active(true);
    }

    /// Go down one in the list of results (increasing the index)
    pub fn down(&mut self) {
        self.set_current_result_active(false);
        self.nav.down();
        self.set_current_result_active(true);
    }

    /// Go up one in the list of results (decrease the index)
    pub fn up(&mut self) {
        self.set_current_result_active(false);
        self.nav.up();
        self.set_current_result_active(true);
    }

    /// Gets the selected result
    pub fn get_result(&self) -> Option<&ResultEntry> {
        let index = self.nav.selected()?;

        Some(&self.categories[index.category].entries[index.entry])
    }

    /// Get the underlying [`Self::categories`] widget
    pub const fn results_widget(&self) -> &gtk::Box {
        self.categories.widget()
    }

    /// Helper method to set the result at the current selected index as active
    fn set_current_result_active(&self, active: bool) {
        let Some(index) = self.nav.selected() else {
            return;
        };

        self.categories.send(
            index.category,
            ResultCategoryInput::EntryMessage(index.entry, ResultEntryInput::SetActive(active)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- NavigationState tests

    #[test]
    fn nav_empty_list() {
        let nav = NavigationState::new(100);
        assert!(nav.is_empty());
        assert!(nav.selected().is_none());
        assert!(nav.min_index().is_none());
        assert!(nav.max_index().is_none());
    }

    #[test]
    fn nav_set_results_selects_first() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 3]);

        assert!(!nav.is_empty());
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_set_results_clears_previous() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![3]);
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.down();
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 2)));

        nav.set_results(vec![1]);
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_set_results_empty_vec_clears() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![3]);
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.set_results(vec![]);
        assert!(nav.is_empty());
        assert!(nav.selected().is_none());
    }

    // -- down tests --

    #[test]
    fn nav_down_single_entry_wraps() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![1]);

        nav.down();
        // Wraps back to the only entry
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_down_single_category_multiple_entries() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![3]);

        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 1)));

        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 2)));

        // Wraps back to first
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_down_crosses_category_boundary() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 2]);

        // Start at (0, 0)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.down(); // (0, 1)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 1)));

        nav.down(); // crosses to (1, 0)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 0)));

        nav.down(); // (1, 1)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 1)));

        // Wraps back to top
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_down_wraps_at_bottom() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![1, 1]);

        nav.down(); // (1, 0)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 0)));

        nav.down(); // wraps to (0, 0)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_down_respects_max_entries() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![2, 2]);

        // max_entries is 3, so valid indices are (0,0), (0,1), (1,0)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.down(); // (0, 1)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 1)));

        nav.down(); // (1, 0)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 0)));

        // (1, 1) would be row 3 >= max_entries, so wraps to (0, 0)
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_down_respects_max_entries_single_category() {
        let mut nav = NavigationState::new(2);
        nav.set_results(vec![5]);

        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.down(); // (0, 1)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 1)));

        // (0, 2) would be row 2 >= max_entries, so wraps to (0, 0)
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_down_noop_on_empty() {
        let mut nav = NavigationState::new(100);
        nav.down(); // no-op
        assert!(nav.selected().is_none());
    }

    // -- up tests --

    #[test]
    fn nav_up_single_entry_wraps() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![1]);

        nav.up();
        // Wraps back to the only entry (which is also the last)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_up_single_category_multiple_entries() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![3]);

        // Start at (0, 0), up wraps to last
        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 2)));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 1)));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        // Wraps back to last
        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 2)));
    }

    #[test]
    fn nav_up_crosses_category_boundary() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 2]);

        // Move to (1, 0)
        nav.down();
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 0)));

        // Up goes to last entry of previous category: (0, 1)
        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 1)));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        // Wraps to bottom: (1, 1)
        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 1)));
    }

    #[test]
    fn nav_up_wraps_at_top() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![1, 1]);

        // At (0, 0), up wraps to bottom: (1, 0) since category 1 has 1 entry
        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 0)));
    }

    #[test]
    fn nav_up_wraps_at_top_with_max_entries() {
        let mut nav = NavigationState::new(5);
        nav.set_results(vec![5, 10]);

        // At (0, 0), up wraps to bottom: max_index is (0, 4) since max_entries=5 clips to category 0
        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 4)));
    }

    #[test]
    fn nav_up_noop_on_empty() {
        let mut nav = NavigationState::new(100);
        nav.up(); // no-op
        assert!(nav.selected().is_none());
    }

    // -- min_index / max_index tests --

    #[test]
    fn nav_min_max_single_category() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![5]);

        assert_eq!(nav.min_index(), Some(ResultIndex::new(0, 0)));
        assert_eq!(nav.max_index(), Some(ResultIndex::new(0, 4)));
    }

    #[test]
    fn nav_min_max_multiple_categories() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 3]);

        assert_eq!(nav.min_index(), Some(ResultIndex::new(0, 0)));
        assert_eq!(nav.max_index(), Some(ResultIndex::new(1, 2)));
    }

    #[test]
    fn nav_min_max_with_max_entries() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![2, 2]);

        assert_eq!(nav.min_index(), Some(ResultIndex::new(0, 0)));
        // max_entries = 3: i=0 row=2 < 3, i=1 row=4 >= 3, overshoot=1, entry=2-1-1=0
        assert_eq!(nav.max_index(), Some(ResultIndex::new(1, 0)));
    }

    #[test]
    fn nav_max_index_exactly_at_limit() {
        let mut nav = NavigationState::new(5);
        nav.set_results(vec![5]);

        assert_eq!(nav.max_index(), Some(ResultIndex::new(0, 4)));
    }

    #[test]
    fn nav_max_index_total_less_than_max() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![1, 2]);

        assert_eq!(nav.max_index(), Some(ResultIndex::new(1, 1)));
    }

    // -- round-trip tests (up then down, down then up) --

    #[test]
    fn nav_round_trip_down_then_up_at_wrap() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![3, 3]);

        // Start at last valid index, down wraps to first
        let last = nav.max_index().unwrap();
        nav.selected = Some(last.clone());

        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.up();
        assert_eq!(nav.selected(), Some(&last));
    }

    #[test]
    fn nav_round_trip_up_then_down_at_wrap() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![3, 3]);

        // Start at first index, up wraps to last
        let first = ResultIndex::new(0, 0);
        nav.selected = Some(first.clone());

        nav.up();
        let last = nav.max_index().unwrap();
        assert_eq!(nav.selected(), Some(&last));

        nav.down();
        assert_eq!(nav.selected(), Some(&first));
    }

    #[test]
    fn nav_round_trip_from_middle() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![3, 3]);

        // Move to middle of category 1 (across boundary)
        nav.down();
        nav.down();
        nav.down();
        let original = nav.selected().cloned().unwrap();

        nav.down();
        nav.up();
        assert_eq!(nav.selected(), Some(&original));

        nav.up();
        nav.down();
        assert_eq!(nav.selected(), Some(&original));
    }

    #[test]
    fn nav_round_trip_at_category_boundary() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 2]);

        // Move to first entry of category 1
        nav.down();
        nav.down();
        let original = nav.selected().cloned().unwrap();

        nav.down();
        nav.up();
        assert_eq!(nav.selected(), Some(&original));

        nav.up();
        nav.down();
        assert_eq!(nav.selected(), Some(&original));
    }

    #[test]
    fn nav_full_traverse_down() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 2]);

        let order = [
            ResultIndex::new(0, 0),
            ResultIndex::new(0, 1),
            ResultIndex::new(1, 0),
            ResultIndex::new(1, 1),
        ];

        for expected in &order {
            assert_eq!(nav.selected(), Some(expected));
            nav.down();
        }

        // Wrapped back to start
        assert_eq!(nav.selected(), Some(&order[0]));
    }

    #[test]
    fn nav_full_traverse_up() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 2]);

        // Walk all the way up (wrapping to bottom first)
        nav.up(); // wraps to (1, 1)
        let order = [
            ResultIndex::new(1, 1),
            ResultIndex::new(1, 0),
            ResultIndex::new(0, 1),
            ResultIndex::new(0, 0),
        ];

        for expected in &order {
            assert_eq!(nav.selected(), Some(expected));
            nav.up();
        }

        // Wrapped back to (1, 1)
        assert_eq!(nav.selected(), Some(&order[0]));
    }

    #[test]
    fn nav_row_of() {
        let mut nav = NavigationState::new(100);
        nav.set_results(vec![2, 3, 1]);

        assert_eq!(nav.row_of(&ResultIndex::new(0, 0)), 0);
        assert_eq!(nav.row_of(&ResultIndex::new(0, 1)), 1);
        assert_eq!(nav.row_of(&ResultIndex::new(1, 0)), 2);
        assert_eq!(nav.row_of(&ResultIndex::new(1, 2)), 4);
        assert_eq!(nav.row_of(&ResultIndex::new(2, 0)), 5);
    }

    #[test]
    fn nav_row_of_at_max_entries_boundary() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![5, 5]);

        // (0, 2) is the last valid index (row 2 < 3)
        assert_eq!(nav.row_of(&ResultIndex::new(0, 2)), 2);
        // (0, 3) is beyond max_entries (row 3 >= 3)
        assert_eq!(nav.row_of(&ResultIndex::new(0, 3)), 3);
        // (1, 0) is also beyond max_entries
        assert_eq!(nav.row_of(&ResultIndex::new(1, 0)), 5);
    }

    // -- max_entries edge cases --

    #[test]
    fn nav_single_max_entry() {
        let mut nav = NavigationState::new(1);
        nav.set_results(vec![3, 3]);

        // Only (0, 0) is valid
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
        assert_eq!(nav.max_index(), Some(ResultIndex::new(0, 0)));

        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_down_from_max_index_wraps_to_min() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![2, 2]);

        // max_index is (1, 0) — the only valid index in category 1
        assert_eq!(nav.max_index(), Some(ResultIndex::new(1, 0)));
        nav.selected = Some(ResultIndex::new(1, 0));

        nav.down();
        // (1, 1) would be row 3 >= max_entries, wraps to (0, 0)
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    #[test]
    fn nav_up_from_min_index_wraps_to_max() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![2, 2]);

        assert_eq!(nav.min_index(), Some(ResultIndex::new(0, 0)));
        nav.selected = Some(ResultIndex::new(0, 0));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(1, 0)));
    }

    #[test]
    fn nav_up_with_max_entries_clipping_mid_category() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![5, 5]);

        // Only (0,0), (0,1), (0,2) are valid
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 2)));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 1)));

        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));

        // Wraps back to (0, 2)
        nav.up();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 2)));
    }

    #[test]
    fn nav_set_results_single_category_exceeds_max_entries() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![10]);

        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
        assert_eq!(nav.max_index(), Some(ResultIndex::new(0, 2)));

        nav.down();
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 2)));

        // Wraps
        nav.down();
        assert_eq!(nav.selected(), Some(&ResultIndex::new(0, 0)));
    }

    // -- up() never exceeds max_entries --

    #[test]
    fn nav_up_never_exceeds_max_entries() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![5, 5]);

        for _ in 0..20 {
            nav.up();
            if let Some(idx) = nav.selected() {
                assert!(
                    nav.row_of(idx) < nav.max_entries,
                    "up() produced index {:?} at row {} which is >= max_entries {}",
                    idx,
                    nav.row_of(idx),
                    nav.max_entries,
                );
            }
        }
    }

    #[test]
    fn nav_down_never_exceeds_max_entries() {
        let mut nav = NavigationState::new(3);
        nav.set_results(vec![5, 5]);

        for _ in 0..20 {
            nav.down();
            if let Some(idx) = nav.selected() {
                assert!(
                    nav.row_of(idx) < nav.max_entries,
                    "down() produced index {:?} at row {} which is >= max_entries {}",
                    idx,
                    nav.row_of(idx),
                    nav.max_entries,
                );
            }
        }
    }
}
