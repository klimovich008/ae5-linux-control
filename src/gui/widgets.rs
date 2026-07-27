//! Widget primitives shared across pages.

use gtk::prelude::*;

/// A titled card with an index badge, description and an action row.
pub fn profile_card(index: &str, title: &str, description: &str, actions: &gtk::Box) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.add_css_class("profile-card");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let index = gtk::Label::new(Some(index));
    index.add_css_class("section-index");
    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    title.add_css_class("section-title");
    heading.append(&index);
    heading.append(&title);
    card.append(&heading);

    let description = gtk::Label::new(Some(description));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("dim-label");
    card.append(&description);
    card.append(actions);
    card
}
