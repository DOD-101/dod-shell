//! The results of the search
//!
//! The main item of the modle is [LauncherResults].
//! It wraps a [relm4::prelude::FactoryVecDeque], which contains the actual results and additionally stores the
//! currently selected item's index
use relm4::prelude::*;

/// See module level documentation
#[derive(Debug)]
pub struct LauncherResults {
    /// The results
    results: FactoryVecDeque<LauncherResult>,
    /// The index of the currently selected result
    selected_index: usize,
}

/// An individual result
#[derive(Debug)]
pub struct LauncherResult {
    /// The label of the result (aka. what the user sees)
    label: String,
    // Whether the result is currently selected
    active: bool,
}

/// Input messages for [LauncherResult]
#[derive(Debug)]
pub enum LauncherResultInput {
    /// Set a [LauncherResult] as active
    ResultActive,
    /// Set a [LauncherResult] as inactive
    ResultInactive,
}

/// Widget associated with the [LauncherResult] component
///
/// Generated with [macro@relm4::component].
#[relm4::factory(pub)]
impl FactoryComponent for LauncherResult {
    type Init = String;
    type Input = LauncherResultInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[name(launch_option_label)]
        gtk::Label {
            set_label: &self.label,
            #[watch]
            set_class_active: ("active", self.active),
        }
    }

    fn init_model(label: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self {
            label,
            active: false,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            LauncherResultInput::ResultActive => {
                self.active = true;
            }
            LauncherResultInput::ResultInactive => {
                self.active = false;
            }
        }
    }
}

impl Default for LauncherResults {
    fn default() -> Self {
        Self {
            results: FactoryVecDeque::builder()
                .launch(gtk::Box::default())
                .detach(),
            selected_index: 0,
        }
    }
}

impl LauncherResults {
    /// Set [Self::results]
    pub fn set_results(&mut self, results: Vec<String>) {
        {
            let mut guard = self.results.guard();

            guard.clear();

            for result in results {
                guard.push_back(result);
            }
        }

        self.selected_index = 0;
        self.set_current_result_active();
    }

    /// Get the underlying [Self::results] widget
    pub fn results_widget(&self) -> &gtk::Box {
        self.results.widget()
    }

    /// Increase the [Self::selected_index] by one and update the results active state accordingly
    pub fn increase_active(&mut self) {
        self.set_current_result_inactive();

        // increase the index
        if let Some(max_selected_index) = self.results.len().checked_sub(1) {
            self.selected_index += 1;

            if self.selected_index > max_selected_index {
                self.selected_index = 0;
            }
        }

        self.set_current_result_active();
    }

    /// Decrease the [Self::selected_index] by one and update the results active state accordingly
    pub fn decrease_active(&mut self) {
        self.set_current_result_inactive();

        if let Some(max_selected_index) = self.results.len().checked_sub(1) {
            self.selected_index = self
                .selected_index
                .checked_sub(1)
                .unwrap_or(max_selected_index);
        }

        self.set_current_result_active();
    }

    /// Set the result at the current [Self::selected_index] as active
    fn set_current_result_active(&self) {
        if self.results.is_empty() {
            return;
        }

        self.results
            .send(self.selected_index, LauncherResultInput::ResultActive);
    }

    /// Set the result at the current [Self::selected_index] as inactive
    fn set_current_result_inactive(&self) {
        if self.results.is_empty() {
            return;
        }

        self.results
            .send(self.selected_index, LauncherResultInput::ResultInactive);
    }

    /// Get the [Self::selected_index]
    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }
}
