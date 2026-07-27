//! Scout Mode page: explicitly unsupported, documented rather than faked.

use gtk::prelude::*;

use crate::gui::widgets::profile_card;

pub fn scout_page() -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Scout Mode"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    let status = gtk::Label::new(Some("UNAVAILABLE IN LINUX"));
    status.add_css_class("unavailable-pill");
    heading.append(&status);
    page.append(&heading);

    let explanation = gtk::Label::new(Some(
        "Scout Mode is visible here so Windows users can account for the feature during \
         migration. The Creative implementation and its hotkey integration are proprietary, \
         and the Linux CA0132 driver does not expose an equivalent control.",
    ));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.add_css_class("dim-label");
    page.append(&explanation);

    let alternatives = gtk::Box::new(gtk::Orientation::Vertical, 8);
    for text in [
        "Use the Equalizer page for a transparent, user-controlled frequency emphasis.",
        "Use a PipeWire filter-chain preset only when you can verify its gain and latency.",
        "Imported Windows profiles retain Scout Mode as an explicit unsupported item.",
    ] {
        let row = gtk::Label::new(Some(&format!("• {text}")));
        row.set_xalign(0.0);
        row.set_wrap(true);
        alternatives.append(&row);
    }
    page.append(&profile_card(
        "STATUS",
        "Linux status: unavailable",
        "AE-5 Control does not present a decorative switch as working hardware support.",
        &alternatives,
    ));

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}
