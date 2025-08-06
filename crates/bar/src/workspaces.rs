use hyprland::{
    dispatch,
    dispatch::{Dispatch, DispatchType},
};

use gtk::prelude::*;
use relm4::{gtk::prelude::ButtonExt, prelude::*};

#[derive(Debug)]
pub struct Workspaces {
    pub workspaces: FactoryVecDeque<WorkspaceButton>,
}
impl Default for Workspaces {
    fn default() -> Self {
        Self {
            workspaces: FactoryVecDeque::builder()
                .launch(gtk::Box::default())
                .detach(),
        }
    }
}

#[derive(Debug)]
pub enum WorkspacesMsg {
    UpdateActiveWorkspace(i32),
}

#[relm4::component(pub)]
impl SimpleComponent for Workspaces {
    type Init = Vec<i32>;
    type Input = WorkspacesMsg;
    type Output = ();

    view! {
        #[name(workspaces)]
        gtk::Box {
            set_css_classes: &["workspaces"],
            #[local_ref]
            workspace_box -> gtk::Box {
                set_css_classes: &["__workspaces_inner"],
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

#[derive(Debug, Default)]
pub struct WorkspaceButton {
    number: i32,
    active: bool,
}

#[derive(Debug)]
pub enum WorkspaceButtonMsg {
    Clicked,
}

#[relm4::factory(pub)]
impl FactoryComponent for WorkspaceButton {
    type Init = i32;
    type Input = WorkspaceButtonMsg;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[name(workspace_btn)]
        gtk::Button {
            set_css_classes: &["workspace-btn"],
            set_label: self.number.to_string().as_str(),
            connect_clicked => WorkspaceButtonMsg::Clicked,
            #[watch]
            set_class_active: ("active", self.active),
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
