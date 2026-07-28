use std::fmt;

const FEATURE_PARITY_TSV: &str = include_str!("../feature-parity.tsv");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureSupport {
    Verified,
    Substituted,
    Deferred,
    Unsupported,
}

impl FeatureSupport {
    pub const ALL: [Self; 4] = [
        Self::Verified,
        Self::Substituted,
        Self::Deferred,
        Self::Unsupported,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "verified" => Some(Self::Verified),
            "intentionally substituted" | "substituted" => Some(Self::Substituted),
            "deferred" => Some(Self::Deferred),
            "unsupported" => Some(Self::Unsupported),
            _ => None,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Verified => "Works and has passed its current acceptance evidence",
            Self::Substituted => "Uses a documented Linux-native equivalent",
            Self::Deferred => "Implemented or exposed, but still needs acceptance evidence",
            Self::Unsupported => "Has no verified safe Linux mechanism",
        }
    }
}

impl fmt::Display for FeatureSupport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Verified => "verified",
            Self::Substituted => "intentionally substituted",
            Self::Deferred => "deferred",
            Self::Unsupported => "unsupported",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureParity {
    pub area: &'static str,
    pub feature: &'static str,
    pub support: FeatureSupport,
    pub linux_mechanism: &'static str,
    pub current_evidence: &'static str,
    pub remaining_gate: &'static str,
    pub source: &'static str,
}

pub fn feature_parity() -> impl Iterator<Item = FeatureParity> {
    FEATURE_PARITY_TSV.lines().skip(1).map(parse_row)
}

fn parse_row(line: &'static str) -> FeatureParity {
    let mut fields = line.split('\t');
    let row = FeatureParity {
        area: fields.next().expect("feature parity area"),
        feature: fields.next().expect("feature parity name"),
        support: FeatureSupport::parse(fields.next().expect("feature parity status"))
            .expect("known feature parity status"),
        linux_mechanism: fields.next().expect("feature parity Linux mechanism"),
        current_evidence: fields.next().expect("feature parity evidence"),
        remaining_gate: fields.next().expect("feature parity remaining gate"),
        source: fields.next().expect("feature parity source"),
    };
    assert!(
        fields.next().is_none(),
        "feature parity rows must contain seven fields"
    );
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn embeds_every_validated_feature_once() {
        let features = feature_parity().collect::<Vec<_>>();
        assert_eq!(features.len(), FEATURE_PARITY_TSV.lines().count() - 1);
        assert!(
            FeatureSupport::ALL
                .into_iter()
                .all(|support| { features.iter().any(|feature| feature.support == support) })
        );

        let unique = features
            .iter()
            .map(|feature| (feature.area, feature.feature))
            .collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), features.len());
        assert!(features.iter().all(|feature| {
            !feature.area.is_empty()
                && !feature.feature.is_empty()
                && !feature.linux_mechanism.is_empty()
                && !feature.current_evidence.is_empty()
                && !feature.remaining_gate.is_empty()
                && !feature.source.is_empty()
        }));
    }

    #[test]
    fn accepts_cli_status_names() {
        assert_eq!(
            FeatureSupport::parse("substituted"),
            Some(FeatureSupport::Substituted)
        );
        for support in FeatureSupport::ALL {
            assert_eq!(FeatureSupport::parse(&support.to_string()), Some(support));
        }
        assert_eq!(FeatureSupport::parse("unknown"), None);
    }
}
