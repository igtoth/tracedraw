//! Stable identifiers for document objects.
//!
//! Ids are plain integers, unique within a document, never reused. They are
//! what commands, the UI and the control channel refer to; indices are not
//! stable across edits, ids are.

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);

        impl $name {
            pub const fn raw(self) -> u64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.0)
            }
        }
    };
}

id_type!(PageId);
id_type!(LayerId);
id_type!(ShapeId);

/// Monotonic id source owned by a document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdSource {
    next: u64,
}

impl IdSource {
    /// The id the next allocation will return, without allocating.
    pub fn peek_next(&self) -> u64 {
        self.next + 1
    }

    pub fn next(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    pub fn page(&mut self) -> PageId {
        PageId(self.next())
    }

    pub fn layer(&mut self) -> LayerId {
        LayerId(self.next())
    }

    pub fn shape(&mut self) -> ShapeId {
        ShapeId(self.next())
    }
}
