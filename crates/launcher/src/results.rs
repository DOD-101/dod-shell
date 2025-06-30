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
    pub results: FactoryVecDeque<LauncherResult>,
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

// WARN: Not sure I fully love this api, since it seems rather error-prone
impl LauncherResults {
    /// Increase the [Self::selected_index] by one
    fn increase(&mut self) {
        if let Some(max_selected_index) = self.results.len().checked_sub(1) {
            self.selected_index += 1;

            if self.selected_index > max_selected_index {
                self.selected_index = 0;
            }
        }
    }

    /// Decrease the [Self::selected_index] by one
    fn decrease(&mut self) {
        if let Some(max_selected_index) = self.results.len().checked_sub(1) {
            self.selected_index = self
                .selected_index
                .checked_sub(1)
                .unwrap_or(max_selected_index);
        }
    }

    /// Set the result at the [Self::selected_index] as active
    fn set_active_result(&self) {
        if self.results.is_empty() {
            return;
        }

        self.results
            .send(self.selected_index, LauncherResultInput::ResultActive);
    }

    /// Set the result at the [Self::selected_index] as inactive
    fn set_inactive_result(&self) {
        if self.results.is_empty() {
            return;
        }

        self.results
            .send(self.selected_index, LauncherResultInput::ResultInactive);
    }

    /// Increase the [Self::selected_index] by one and update the results active state accordingly
    pub fn increase_and_set(&mut self) {
        self.set_inactive_result();

        self.increase();

        self.set_active_result();
    }

    /// Decrease the [Self::selected_index] by one and update the results active state accordingly
    pub fn decrease_and_set(&mut self) {
        self.set_inactive_result();

        self.decrease();

        self.set_active_result();
    }

    /// Reset the [Self::selected_index] to 0 and update the results active state accordingly
    pub fn reset_and_set(&mut self) {
        self.set_inactive_result();

        self.selected_index = 0;

        self.set_active_result();
    }

    /// Get the [Self::selected_index]
    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }
}
