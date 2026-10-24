//! The evidence manifest, embedded image files, and round gating.

use std::fmt::{self, Display, Formatter};

use rust_embed::Embed;

/// The image files under `Evidence/`, embedded into the binary.
#[derive(Embed)]
#[folder = "Evidence/"]
pub struct EvidenceFiles;

/// A game round, always within `Round::MIN..=Round::MAX`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Round(u8);

impl Round {
    /// The first round.
    pub const MIN: Round = Round(1);
    /// The last round.
    pub const MAX: Round = Round(4);

    /// Parses a round number, rejecting anything outside the game's rounds.
    pub fn new(n: u8) -> Option<Round> {
        (Self::MIN.0..=Self::MAX.0).contains(&n).then_some(Round(n))
    }

    /// The round number.
    pub fn get(self) -> u8 {
        self.0
    }

    /// Every round in order.
    pub fn all() -> impl Iterator<Item = Round> {
        (Self::MIN.0..=Self::MAX.0).map(Round)
    }
}

impl Display for Round {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// One piece of evidence, revealed from `round` onwards.
pub struct Item {
    pub round: Round,
    /// URL-safe file name used in routes.
    pub slug: &'static str,
    pub title: &'static str,
    /// Exact file name inside `Evidence/`.
    pub file: &'static str,
}

/// All evidence, in reveal order.
pub static EVIDENCE: [Item; 4] = [
    Item {
        round: Round(2),
        slug: "round2-notebook.jpg",
        title: "The Notebook",
        file: "Round 2 - Notebook.jpg",
    },
    Item {
        round: Round(3),
        slug: "round3-note.jpg",
        title: "The Note",
        file: "Round 3 - Note.jpg",
    },
    Item {
        round: Round(4),
        slug: "round4-view-1.jpg",
        title: "The Study, view 1",
        file: "Round 4 - View 1.jpg",
    },
    Item {
        round: Round(4),
        slug: "round4-view-2.jpg",
        title: "The Study, view 2",
        file: "Round 4 - View 2.jpg",
    },
];

/// Whether `item` has been revealed by round `current`.
pub fn visible(item: &Item, current: Round) -> bool {
    item.round <= current
}

/// Looks up an item by its route slug.
pub fn find(slug: &str) -> Option<&'static Item> {
    EVIDENCE.iter().find(|item| item.slug == slug)
}

/// The items that are first revealed in `round`.
pub fn for_round(round: Round) -> impl Iterator<Item = &'static Item> {
    EVIDENCE.iter().filter(move |item| item.round == round)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn round_new_rejects_0_and_5() {
        assert!(Round::new(0).is_none());
        assert!(Round::new(5).is_none());
        assert_eq!(Round::new(1), Some(Round::MIN));
        assert_eq!(Round::new(4), Some(Round::MAX));
    }

    #[test]
    fn visible_is_round_le_current() {
        let item = &EVIDENCE[0];
        assert!(!visible(item, Round::MIN));
        assert!(visible(item, Round::new(2).unwrap()));
        assert!(visible(item, Round::MAX));
    }

    #[test]
    fn find_known_and_unknown_slug() {
        assert_eq!(find("round3-note.jpg").map(|i| i.title), Some("The Note"));
        assert!(find("nope.jpg").is_none());
    }

    #[test]
    fn every_manifest_file_is_embedded() {
        for item in &EVIDENCE {
            assert!(EvidenceFiles::get(item.file).is_some(), "{}", item.file);
        }
    }

    #[test]
    fn slugs_are_unique() {
        let slugs: HashSet<_> = EVIDENCE.iter().map(|i| i.slug).collect();
        assert_eq!(slugs.len(), EVIDENCE.len());
    }

    #[test]
    fn for_round_and_all() {
        assert_eq!(Round::all().count(), 4);
        assert_eq!(for_round(Round::new(4).unwrap()).count(), 2);
        assert_eq!(Round::MAX.to_string(), "4");
    }
}
