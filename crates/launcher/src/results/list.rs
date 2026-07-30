//! Module for the top-level [`ResultList`] component.
//!
//! It holds all results in a flat [`gio::ListStore`] and displays them
//! in a virtualized [`gtk::ListView`] that only creates widgets for visible items.
//!
//! ## Navigation
//!
//! Navigation is non-wrapping:
//! - Down at the last item: no-op
//! - Up at the first item: no-op
//! - Otherwise: moves selection by one position
//!
//! The [`gtk::ListView`] scrolls automatically to keep the selection visible.
use std::collections::HashSet;

use common::{config::launcher::LauncherConfig, css::Class};
use relm4::{
    Sender,
    gtk::{self, gio, glib::BoxedAnyObject, prelude::*},
    prelude::*,
};

use super::ResultEntry;

/// Default height estimate for a row before we get a measurement
const DEFAULT_ROW_HEIGHT: i32 = 19;
/// Default height estimate for a header before we get a measurement
const DEFAULT_HEADER_HEIGHT: i32 = 19;

/// The result list component using a [`gtk::ListView`].
pub struct ResultList {
    /// Flat list store containing all result items
    store: gio::ListStore,
    /// List widget to display the results
    list_view: gtk::ListView,
    /// Scrollable window around the list
    scrolled_window: gtk::ScrolledWindow,
    /// Selection model managing the single active selection
    selection: gtk::SingleSelection,
    /// Maximum height of the results list
    max_height: i32,
    /// Height of a single entry row
    entry_height: i32,
    /// Height of a single category header row
    header_height: i32,
    /// Number of categories
    categories: i32,
}

impl ResultList {
    /// Set [`Self::store`], overriding any old values entirely.
    fn set_results(&self, entries: Vec<ResultEntry>) {
        self.store.remove_all();

        if !entries.is_empty() {
            self.store.extend_from_slice(
                &entries
                    .into_iter()
                    .map(BoxedAnyObject::new)
                    .collect::<Vec<BoxedAnyObject>>(),
            );
            self.selection.set_selected(0);
            self.scroll(0);
        }
    }

    /// Move selection down by one.
    ///
    /// No-op if at the last item.
    fn down(&self) {
        let pos = self.selection.selected();
        if pos == gtk::INVALID_LIST_POSITION {
            return;
        }
        let next = pos + 1;
        if next < self.store.n_items() {
            self.selection.set_selected(next);
            self.scroll(1);
        }
    }

    /// Move selection up by one.
    ///
    /// No-op if at the first item (row 0).
    fn up(&self) {
        let pos = self.selection.selected();
        if pos == gtk::INVALID_LIST_POSITION {
            return;
        }
        self.selection.set_selected(pos.saturating_sub(1));

        self.scroll(-1);
    }

    /// Get the currently selected result, if any.
    ///
    /// Returns the result by value since items are stored inside
    /// [`glib::BoxedAnyObject`] wrappers.
    pub fn get_result(&self) -> Option<ResultEntry> {
        let pos = self.selection.selected();
        if pos == gtk::INVALID_LIST_POSITION {
            return None;
        }

        let obj = self.store.item(pos)?;
        let flat = obj.downcast_ref::<BoxedAnyObject>()?;
        Some(flat.borrow::<ResultEntry>().clone())
    }

    /// Helper method for scrolling the result list's scrolled window
    ///
    /// The offset ensures that when scrolling down, one extra entry remains visible below the
    /// selection; when scrolling up, one extra entry remains visible above it.
    fn scroll(&self, offset: i32) {
        let mut pos = self.selection.selected();

        if pos == 0 {
            let adj = self.scrolled_window.vadjustment();
            // we add this as a callback, since gtk first needs to compute the layout
            gtk::glib::source::idle_add_local(move || {
                adj.set_value(0.0);
                gtk::glib::ControlFlow::Break
            });
            return;
        }

        if offset.is_positive() {
            pos = pos.saturating_add(offset.unsigned_abs());
        } else {
            pos = pos.saturating_sub(offset.unsigned_abs());
        }

        self.list_view.scroll_to(
            pos.min(self.store.n_items().saturating_sub(1)),
            gtk::ListScrollFlags::NONE,
            None,
        );
    }

    /// Computes the height for [`Self::scrolled_window`]
    ///
    /// ## Performance
    ///
    /// We make sure to only iterate the list of entries to count the headers if the total height,
    /// given just the rows, is less than `self.max_height`. In practice this means iteration of the
    /// entries only occurs if the list is short.
    #[allow(
        clippy::cast_possible_wrap,
        reason = "There will never be that many entries"
    )]
    fn update_list_height(&self) {
        let total =
            self.store.n_items() as i32 * self.entry_height + self.header_height * self.categories;

        self.scrolled_window
            .set_height_request(total.min(self.max_height));
    }
}

/// Input messages for [`ResultList`].
#[derive(Debug)]
pub enum ResultListInput {
    /// Set the results displayed in the list, overriding any previous results.
    SetResults(Vec<ResultEntry>),
    /// Get the currently selected result
    ///
    /// The result is then sent back as an output message
    GetResult,
    /// Move the selection up by one.
    Up,
    /// Move the selection down by one.
    Down,
    /// Sent by the first category header to accurately know it's height
    CategoryHeaderHeight(i32),
    /// Sent by the entry to accurately know it's height
    EntryHeight(i32),
}

/// Output messages for [`ResultList`].
#[derive(Debug)]
pub enum ResultListOuput {
    /// Sent when the search is finished
    ///
    /// Contains the selected result, if one exists.
    Result(Option<ResultEntry>),
    /// Currently selected index
    Selected(u32),
}

/// Widget associated with the [`ResultList`] component.
#[relm4::component(pub)]
impl Component for ResultList {
    type Init = LauncherConfig;
    type Input = ResultListInput;
    type Output = ResultListOuput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            add_css_class: Class::ResultBox.as_ref(),

            /// ScrolledWindow to only show a subset of results
            #[local_ref]
            scrolled_window -> gtk::ScrolledWindow {
                set_vscrollbar_policy: gtk::PolicyType::Automatic,
                set_hscrollbar_policy: gtk::PolicyType::Never,

                /// List containing the results
                #[local_ref]
                list_view -> gtk::ListView {
                    connect_activate[sender] => move |_, _| sender.input(ResultListInput::GetResult),
                    add_css_class: Class::ResultList.as_ref(),
                }
            }
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let store = gio::ListStore::new::<BoxedAnyObject>();

        let (list_view, selection) =
            build_list_view(&store, &config, sender.input_sender().clone());

        let scrolled_window = gtk::ScrolledWindow::new();

        selection.connect_selected_notify({
            let sender = sender.output_sender().clone();

            move |selection| {
                let index = selection.selected();
                if index != gtk::INVALID_LIST_POSITION {
                    sender.emit(ResultListOuput::Selected(index));
                }
            }
        });

        let model = Self {
            store,
            max_height: config.results_height,
            selection,
            list_view: list_view.clone(),
            scrolled_window: scrolled_window.clone(),
            entry_height: DEFAULT_ROW_HEIGHT,
            header_height: DEFAULT_HEADER_HEIGHT,
            categories: 0,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        if matches!(
            msg,
            ResultListInput::CategoryHeaderHeight(_) | ResultListInput::EntryHeight(_)
        ) {
            self.update_list_height();
        }

        match msg {
            #[allow(
                clippy::cast_possible_wrap,
                reason = "There will never be that many entries"
            )]
            ResultListInput::SetResults(entries) => {
                self.categories = {
                    let hash_set: HashSet<&String> = entries
                        .iter()
                        .filter_map(|e| e.category.as_ref().map(|c| &c.name))
                        .collect::<HashSet<_>>();

                    hash_set.len() as i32
                };
                self.set_results(entries);
                self.update_list_height();
            }
            ResultListInput::GetResult => sender
                .output_sender()
                .emit(ResultListOuput::Result(self.get_result())),
            ResultListInput::Up => self.up(),
            ResultListInput::Down => self.down(),
            ResultListInput::CategoryHeaderHeight(h) => self.header_height = h,
            ResultListInput::EntryHeight(h) => self.entry_height = h,
        }
    }
}

/// Helper macro for conversion from [`BoxedAnyObject`] to [`ResultEntry`]
macro_rules! to_entry {
    ($gobj:expr) => {
        $gobj
            .downcast_ref::<BoxedAnyObject>()
            .unwrap()
            .borrow::<ResultEntry>()
    };
    ($($x:expr),+ $(,)?) => {
        (
        $( to_entry!($x) ),+
        )
    }
}

/// Helper function to build [`gtk::ListView`] for results
fn build_list_view(
    store: &gio::ListStore,
    config: &LauncherConfig,
    input_sender: Sender<ResultListInput>,
) -> (gtk::ListView, gtk::SingleSelection) {
    // sorter only needed for the sections (categories)
    let section_sorter = gtk::CustomSorter::new(|obj1, obj2| {
        let (a, b) = to_entry!(obj1, obj2);

        if let (Some(cat_a), Some(cat_b)) = (a.category.clone(), b.category.clone()) {
            return cat_a.name.cmp(&cat_b.name).into();
        }

        gtk::Ordering::Equal
    });
    let sort_model = gtk::SortListModel::new(Some(store.clone()), None::<gtk::CustomSorter>);
    sort_model.set_section_sorter(Some(&section_sorter));

    let selection_model = gtk::SingleSelection::new(Some(sort_model));

    // creates the rows
    let item_factory = gtk::SignalListItemFactory::new();
    let align = if config.center_results {
        gtk::Align::Center
    } else {
        gtk::Align::Start
    };
    let item_bound = std::cell::Cell::new(false);
    let sender = input_sender.clone();
    item_factory.connect_setup(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();

        let label = create_row_label(align);
        item.set_child(Some(&label));

        item.connect_selected_notify(|item| {
            let label = item.child().and_downcast::<gtk::Label>().unwrap();

            label.set_class_active(Class::Active.as_ref(), item.is_selected());
        });

        if !item_bound.replace(true) {
            let sender = sender.clone();
            label.connect_realize(move |w| {
                let height = w.measure(gtk::Orientation::Vertical, -1).1;
                sender.emit(ResultListInput::EntryHeight(height));
            });
        }
    });
    item_factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let obj = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let entry = obj.borrow::<ResultEntry>();
        let label = item.child().and_downcast::<gtk::Label>().unwrap();
        label.set_label(&entry.label);
    });

    // creates the category headers
    let header_factory = gtk::SignalListItemFactory::new();
    let header_bound = std::cell::Cell::new(false);
    header_factory.connect_setup(move |_, header| {
        let label = create_header(align);

        header
            .downcast_ref::<gtk::ListHeader>()
            .unwrap()
            .set_child(Some(&label));

        if !header_bound.replace(true) {
            let sender = input_sender.clone();
            label.connect_realize(move |w| {
                let height = w.measure(gtk::Orientation::Vertical, -1).1;
                sender.emit(ResultListInput::CategoryHeaderHeight(height));
            });
        }
    });
    header_factory.connect_bind(|_, header| {
        let header = header.downcast_ref::<gtk::ListHeader>().unwrap();
        let obj = header.item().and_downcast::<BoxedAnyObject>().unwrap();
        let entry = obj.borrow::<ResultEntry>();
        let label = header.child().and_downcast::<gtk::Label>().unwrap();
        if let Some(ref category) = entry.category {
            label.set_label(&category.name);
            label.set_visible(true);
        } else {
            label.set_visible(false);
        }
    });

    let list_view = gtk::ListView::new(Some(selection_model.clone()), Some(item_factory));
    list_view.set_header_factory(Some(&header_factory));

    (list_view, selection_model)
}

/// Build a single result-row
fn create_row_label(align: gtk::Align) -> gtk::Label {
    relm4::view! {
        label = gtk::Label {
            set_halign: align,
            add_css_class: Class::ResultEntryLabel.as_ref(),
        }
    }
    label
}

/// Build a single category header
fn create_header(align: gtk::Align) -> gtk::Label {
    relm4::view! {
        label = gtk::Label {
            set_halign: align,
            add_css_class: Class::ResultCategoryLabel.as_ref(),
        }
    }
    label
}
