use relm4::prelude::*;

use crate::results::{ResultEntry, category::ResultCategoryInput, entry::ResultEntryInput};

use super::category::ResultCategory;

/// See module level documentation
#[derive(Debug)]
pub struct ResultList {
    /// Categories of results
    categories: FactoryVecDeque<ResultCategory>,
    /// Index of the currently selected result entry
    selected_entry: ResultIndex,
    max_rows: usize,
}

#[derive(Debug, Default)]
struct ResultIndex {
    /// Index of the category
    pub category: usize,
    /// Index of the entry in the category
    pub entry: usize,
}

impl ResultList {
    pub fn new(max_rows: usize) -> Self {
        Self {
            categories: FactoryVecDeque::builder().launch_default().detach(),
            selected_entry: ResultIndex::default(),

            max_rows,
        }
    }

    /// Set [Self::results]
    pub fn set_results(&mut self, categories: Vec<ResultCategory>) {
        {
            let mut guard = self.categories.guard();

            guard.clear();

            let mut row = 0;

            for mut result in categories {
                row += result.len();
                if row > self.max_rows {
                    let overflow = row - self.max_rows;

                    {
                        let mut guard = result.entries.guard();

                        for _ in 0..overflow {
                            guard.pop_back();
                        }
                    }
                }
                guard.push_back(result);

                if row >= self.max_rows {
                    break;
                }
            }
        }

        self.selected_entry = self.first_valid_result_index();
        self.set_current_result_active(true);
    }

    fn first_valid_result_index(&self) -> ResultIndex {
        let category = self
            .categories
            .iter()
            .enumerate()
            .find(|c| !c.1.is_empty())
            .map(|i| i.0);

        match category {
            Some(cat) => ResultIndex {
                category: cat,
                entry: 0,
            },
            None => ResultIndex::default(),
        }
    }

    pub fn get_result(&self) -> Option<&ResultEntry> {
        if self.is_empty() {
            return None;
        }

        Some(&self.current_category().entries[self.selected_entry.entry])
    }

    /// Get the underlying [Self::results] widget
    pub fn results_widget(&self) -> &gtk::Box {
        self.categories.widget()
    }

    /// Set the result at the current [Self::selected_index] as active
    fn set_current_result_active(&self, active: bool) {
        if self.is_empty() {
            return;
        }

        dbg!(&self.selected_entry);

        self.categories.send(
            self.selected_entry.category,
            ResultCategoryInput::EntryMessage(
                self.selected_entry.entry,
                ResultEntryInput::SetActive(active),
            ),
        );
    }
}

impl ResultList {
    /// Go down one in the list of results (increasing the index)
    pub fn down(&mut self) {
        self.set_current_result_active(false);

        if self.compute_row() == self.max_rows {
            self.selected_entry = ResultIndex::default();
        }

        // 1. Try to go down one result in the current category
        if self.current_category().len() > self.selected_entry.entry + 1 {
            self.selected_entry.entry += 1;
        // 2. Go to the first entry in the next category
        } else if self.categories.len() > self.selected_entry.category + 1 {
            self.selected_entry.category += 1;
            self.selected_entry.entry = 0;
        // 3. Go back to the top of the list
        } else {
            self.selected_entry = ResultIndex::default();
        }

        self.set_current_result_active(true);
    }

    /// Go up one in the list of results (decrease the index)
    pub fn up(&mut self) {
        self.set_current_result_active(false);

        // 1. Try to go up one result in the current category
        if self.selected_entry.entry > 0 {
            self.selected_entry.entry -= 1;
        // 2. Go to the last entry in the previous category
        } else if self.selected_entry.category > 0 {
            self.selected_entry.category -= 1;
            self.selected_entry.entry = self.current_category().len().saturating_sub(1);
        // 3. Go back to the bottom of the list
        } else {
            self.selected_entry = self.compute_max_index();
        }

        self.set_current_result_active(true);
    }

    fn current_category(&self) -> &ResultCategory {
        &self.categories[self.selected_entry.category]
    }

    fn compute_row(&self) -> usize {
        let mut row = 0;

        for i in 0..self.selected_entry.category {
            row += self.categories[i].len();
        }

        row
    }

    fn compute_max_index(&self) -> ResultIndex {
        let mut row = 0;

        for i in 0..self.categories.len() {
            row += self.categories[i].len();

            if row >= 40 {
                // row = 44

                let overshoot = row - self.max_rows;

                let entry = self.categories[i].len() - overshoot;

                return ResultIndex { category: i, entry };
            }
        }

        let category = self.categories.len().saturating_sub(1);
        ResultIndex {
            category,
            entry: self.categories[category].len().saturating_sub(1),
        }
    }

    fn is_empty(&self) -> bool {
        self.categories.is_empty() || self.categories.iter().all(|c| c.is_empty())
    }
}
