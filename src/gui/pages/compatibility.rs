//! Compatibility tab: honest per-feature Linux support, sourced from the ledger.

use crate::{FeatureSupport, feature_parity};
use gtk::prelude::*;

pub fn compatibility_page() -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Sound Blaster Command compatibility"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    let count = gtk::Label::new(Some(&format!(
        "{} FEATURES TRACKED",
        feature_parity().count()
    )));
    count.add_css_class("status-pill");
    heading.append(&count);
    page.append(&heading);

    let summary = gtk::Box::new(gtk::Orientation::Vertical, 4);
    for line in compatibility_summary().lines() {
        let label = gtk::Label::new(Some(line));
        label.set_xalign(0.0);
        summary.append(&label);
    }
    page.append(&crate::gui::widgets::profile_card(
        "01",
        "Tracked feature status",
        "This read-only view is built from the same evidence matrix used by the project. A Linux-native equivalent is labeled as a substitution instead of being presented as Creative's implementation.",
        &summary,
    ));

    for (index, support, title, description) in [
        (
            "02",
            FeatureSupport::Unsupported,
            "Unavailable features",
            "No verified safe Linux mechanism exists for these functions. They are listed explicitly instead of appearing as nonfunctional controls.",
        ),
        (
            "03",
            FeatureSupport::Deferred,
            "Pending acceptance",
            "These functions have a Linux control, candidate, or substitute, but still need the stated physical evidence before the project claims full support.",
        ),
    ] {
        let entries = gtk::Box::new(gtk::Orientation::Vertical, 8);
        for feature in feature_parity().filter(|feature| feature.support == support) {
            let expander =
                gtk::Expander::new(Some(&format!("{} · {}", feature.area, feature.feature)));
            expander.add_css_class("feature-entry");

            let details = gtk::Box::new(gtk::Orientation::Vertical, 4);
            for (label, value) in [
                ("Linux mechanism", feature.linux_mechanism),
                ("Current evidence", feature.current_evidence),
                ("Remaining gate", feature.remaining_gate),
                ("Source", feature.source),
            ] {
                let text = gtk::Label::new(Some(&format!("{label}: {value}")));
                text.set_xalign(0.0);
                text.set_wrap(true);
                text.set_selectable(true);
                text.add_css_class("dim-label");
                details.append(&text);
            }
            expander.set_child(Some(&details));
            entries.append(&expander);
        }
        page.append(&crate::gui::widgets::profile_card(
            index,
            title,
            description,
            &entries,
        ));
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

pub fn compatibility_summary() -> String {
    let features = feature_parity().collect::<Vec<_>>();
    let counts = FeatureSupport::ALL.map(|support| {
        features
            .iter()
            .filter(|feature| feature.support == support)
            .count()
    });
    format!(
        "{} tracked features\nVerified: {}\nLinux-native equivalents: {}\nPending acceptance: {}\nUnavailable: {}",
        features.len(),
        counts[0],
        counts[1],
        counts[2],
        counts[3]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_page_summarizes_the_embedded_matrix() {
        let summary = compatibility_summary();
        let features = feature_parity().collect::<Vec<_>>();
        assert!(summary.contains(&format!("{} tracked features", features.len())));
        for (label, support) in [
            ("Verified", FeatureSupport::Verified),
            ("Linux-native equivalents", FeatureSupport::Substituted),
            ("Pending acceptance", FeatureSupport::Deferred),
            ("Unavailable", FeatureSupport::Unsupported),
        ] {
            let count = features
                .iter()
                .filter(|feature| feature.support == support)
                .count();
            assert!(summary.contains(&format!("{label}: {count}")));
        }
    }
}
