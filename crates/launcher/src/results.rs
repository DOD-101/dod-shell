use relm4::prelude::*;

#[derive(Debug)]
pub struct LauncherResults {
    pub results: FactoryVecDeque<LauncherResult>,
    selected_index: usize,
}

#[derive(Debug)]
pub struct LauncherResult {
    label: String,
    active: bool,
}

#[derive(Debug)]
pub enum LauncherResultInput {
    ResultActive,
    ResultInactive,
}

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
    fn decrease(&mut self) {
        if let Some(max_selected_index) = self.results.len().checked_sub(1) {
            self.selected_index = self
                .selected_index
                .checked_sub(1)
                .unwrap_or(max_selected_index);
        }
    }

    fn increase(&mut self) {
        if let Some(max_selected_index) = self.results.len().checked_sub(1) {
            self.selected_index += 1;

            if self.selected_index > max_selected_index {
                self.selected_index = 0;
            }
        }
    }

    fn set_active_result(&self) {
        if self.results.is_empty() {
            return;
        }

        self.results
            .send(self.selected_index, LauncherResultInput::ResultActive);
    }

    fn set_inactive_result(&self) {
        if self.results.is_empty() {
            return;
        }

        self.results
            .send(self.selected_index, LauncherResultInput::ResultInactive);
    }

    pub fn increase_and_set(&mut self) {
        self.set_inactive_result();

        self.increase();

        self.set_active_result();
    }

    pub fn decrease_and_set(&mut self) {
        self.set_inactive_result();

        self.decrease();

        self.set_active_result();
    }

    pub fn reset_and_set(&mut self) {
        self.set_inactive_result();

        self.selected_index = 0;

        self.set_active_result();
    }

    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }
}
