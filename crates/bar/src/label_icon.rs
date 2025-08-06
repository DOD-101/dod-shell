//! Contains custom gtk widget [``LabelIcon``]
//!
//! It contains two labels, one for holding a value the other for displaying an icon after said
//! value.
//!
//! ## Presentation
//!
//! eg: `10%` `󰻠`
//!
//! ## Classes
//!
//! All: "label-icon"
//!
//! Value: "label-icon-label" or "_label"
//!
//! Icon: "label-icon-icon" or "_icon"
use relm4::gtk::{
    self,
    glib::{self, subclass::prelude::*},
};

gtk::glib::wrapper! {
    /// See module level documentation
    pub struct LabelIcon(ObjectSubclass<imp::LabelIcon>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for LabelIcon {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl LabelIcon {
    pub fn set_label(&self, label: &str) {
        let imp = imp::LabelIcon::from_obj(self);

        imp.label.set_label(label);
    }

    pub fn set_icon(&self, icon: &str) {
        let imp = imp::LabelIcon::from_obj(self);

        imp.icon.set_label(icon);
    }
}

/// Implementation Details for [``super::LabelIcon``]
mod imp {
    use relm4::{
        WidgetRef,
        gtk::{
            self,
            glib::{self, subclass::prelude::*},
            prelude::*,
            subclass::prelude::*,
        },
    };

    /// Inner struct for [``super::LabelIcon``]
    #[derive(Debug, Default)]
    pub struct LabelIcon {
        pub(super) label: gtk::Label,
        pub(super) icon: gtk::Label,
    }

    #[gtk::glib::object_subclass]
    impl ObjectSubclass for LabelIcon {
        const NAME: &'static str = "LabelIcon";
        type Type = super::LabelIcon;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.set_layout_manager_type::<gtk::BoxLayout>();

            klass.set_css_name("label-icon");

            klass.set_accessible_role(gtk::AccessibleRole::Label);
        }
    }

    impl ObjectImpl for LabelIcon {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.label.set_parent(obj.widget_ref());
            self.icon.set_parent(obj.widget_ref());

            obj.widget_ref().set_css_classes(&["label-icon"]);
            self.label.set_css_classes(&["label-icon-label", "label"]);
            self.icon.set_css_classes(&["label-icon-icon", "icon"]);
        }

        fn dispose(&self) {
            self.label.unparent();
            self.icon.unparent();
        }
    }

    impl WidgetImpl for LabelIcon {}
}
