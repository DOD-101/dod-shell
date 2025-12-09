//! Workspace Component
//!
//! This [``relm4::SimpleComponent``] displays a list of workspaces and allows clicking on
//! indiviual icons for each to change to that workspace.
//!
//! <div class="warning">
//!
//! ## **Warning**
//!
//! This is designed to work on hyprland and with numbered workspaces.
//! It won't work with another compositor and using named workspaces isn't tested.
//!
//! </div>
use common::{classes, css::Class};
use hyprland::{
    dispatch,
    dispatch::{Dispatch, DispatchType},
};

use gtk::prelude::*;
use relm4::{gtk::prelude::ButtonExt, prelude::*};

/// See module level documentation
#[derive(Debug)]
pub struct Workspaces {
    /// The individual workspace buttons
    pub workspaces: FactoryVecDeque<WorkspaceButton>,
}

// TODO: REMOVE THIS
impl Default for Workspaces {
    fn default() -> Self {
        Self {
            workspaces: FactoryVecDeque::builder()
                .launch(gtk::Box::default())
                .detach(),
        }
    }
}

/// Messages sent to [``Workspaces``]
#[derive(Debug)]
pub enum WorkspacesMsg {
    /// Change wich workspace is shown as the active one
    UpdateActiveWorkspace(i32),
}

#[relm4::component(pub)]
impl SimpleComponent for Workspaces {
    /// A Vector of workspace id's to show
    // TODO: Make this Arc<[i32]>
    type Init = Vec<i32>;
    type Input = WorkspacesMsg;
    type Output = ();

    view! {
        #[name(workspaces)]
        gtk::Box {
            add_css_class: Class::Workspaces.as_ref(),
            #[local_ref]
            workspace_box -> gtk::Box {
                add_css_class: Class::WorkspacesInner.as_ref(),
                set_orientation: gtk::Orientation::Horizontal,
            },
        }
    }

    fn init(
        workspace_ids: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let workspaces = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();

        let mut model = Self { workspaces };

        {
            let mut wps = model.workspaces.guard();

            for i in workspace_ids {
                wps.push_back(i);
            }
        }

        let workspace_box = model.workspaces.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            WorkspacesMsg::UpdateActiveWorkspace(i) => {
                for w in self.workspaces.guard().iter_mut() {
                    w.active = w.number == i;
                }
            }
        }
    }
}

/// A Button for an individual workspace
///
/// Clicking on this will change hyprlands workspace to the associated one.
#[derive(Debug, Default)]
pub struct WorkspaceButton {
    /// The hyprland workspace
    number: i32,
    /// If the workspace is currently the active one
    active: bool,
}

/// Messages sent to [``WorkspaceButton``]
#[derive(Debug)]
pub enum WorkspaceButtonMsg {
    Clicked,
}

#[relm4::factory(pub)]
impl FactoryComponent for WorkspaceButton {
    /// The hyprland workspace of the button
    type Init = i32;
    type Input = WorkspaceButtonMsg;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[name(workspace_btn)]
        gtk::Button {
            set_css_classes: &classes!(Workspace, WorkspaceButton),
            set_label: self.number.to_string().as_str(),
            connect_clicked => WorkspaceButtonMsg::Clicked,
            #[watch]
            set_class_active: (Class::Active.as_ref(), self.active),
        }

    }

    fn init_model(number: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self {
            number,
            active: false,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            WorkspaceButtonMsg::Clicked => {
                if let Err(e) = dispatch!(
                    Workspace,
                    dispatch::WorkspaceIdentifierWithSpecial::Id(self.number)
                ) {
                    log::error!("Error changing workspaces: {e}");
                }
            }
        }
    }
}
