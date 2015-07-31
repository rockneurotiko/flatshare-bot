//! Bot plugin that keeps track of a "to-buy" list. Write "/need item" to add
//! the item to the list. Write "/got item" to remove it.
use std::cmp::{Ord, Ordering};
use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use iter::LangIter;
use itertools::Itertools;
use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};


/// A string wrapper that has a different ordering. It's useful for strings
/// typed by humans who don't care about capitalization. If you write
/// "/need Beer" and "/got beer" afterwards, it still works.
#[derive(Debug, PartialEq, Eq)]
pub struct SimString {
    pub orig: String,
}

impl SimString {
    pub fn new(s: String) -> SimString {
        SimString { orig: s }
    }
}

impl From<String> for SimString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl PartialOrd<SimString> for SimString {
    fn partial_cmp(&self, other: &SimString) -> Option<Ordering> {
        self.to_lowercase().partial_cmp(&other.to_lowercase())
    }
}

impl Ord for SimString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_lowercase().cmp(&other.to_lowercase())
    }
}

// To use all string methods on this type.
impl Deref for SimString {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.orig
    }
}

// Sadly we have to manually implement this again.
impl Display for SimString {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        self.orig.fmt(f)
    }
}

// Works exactly like String::encode and String::decode
impl Encodable for SimString {
    fn encode<E: Encoder>(&self, s: &mut E) -> Result<(), E::Error> {
        self.orig.encode(s)
    }
}

impl Decodable for SimString {
    fn decode<D: Decoder>(s: &mut D) -> Result<Self, D::Error> {
        Ok(SimString {
            orig: try!(String::decode(s)),
        })
    }
}

/// The type keeping track of the list.
#[derive(Default, RustcEncodable, RustcDecodable)]
pub struct Needed {
    pub list: BTreeSet<SimString>,
}

impl Needed {
    /// Method handling a message starting with "/need"
    pub fn handle_need(&mut self, args: String) -> String {
        // Collect items that were already on the list.
        let mut already_there = Vec::new();

        // If the item was already on list: Report. Push to list otherwise.
        for item in Self::split(args) {
            if self.list.contains(&item) {
                already_there.push(item);
            } else {
                self.list.insert(item);
            }
        }

        // Send a nice answer
        let str_already_there = if already_there.is_empty() {
            "".into()
        } else {
            let list = already_there.iter()
                .map(|s| format!("'{}'", s))
                .listify(", ", " and ");
            format!("{} already on the list!\n", list)
        };

        format!("{}We need:\n{}", str_already_there, self.str_list())
    }

    pub fn handle_got(&mut self, args: String) -> String {
        // Collect items that were not on the list.
        let mut not_found = Vec::new();

        // If an item was found: Remove from list. If not found: Report.
        for item in Self::split(args) {
            if self.list.contains(&item) {
                self.list.remove(&item);
            } else {
                not_found.push(item);
            }
        }

        // Send a nice answer
        let str_not_found = if not_found.is_empty() {
            "".into()
        } else {
            let list = not_found.iter()
                .map(|s| format!("'{}'", s))
                .listify(", ", " and ");
            format!("{} not on the list!\n", list)
        };

        if self.list.is_empty() {
            "We have everything we need :-)".into()
        } else {
            format!("{}We still need:\n{}", str_not_found, self.str_list())
        }
    }

    fn split(args: String) -> Vec<SimString> {
        // To obtain the items: Split at comma, remove leading and trailing
        // whitespace, delete empty strings, convert to SimString
        args.split(',')
            .map(|a| a.trim())
            .filter(|a| a.len() > 0)
            .map(|a| a.to_owned().into())
            .collect()
    }

    fn str_list(&self) -> String {
        self.list.iter()
            .enumerate()
            .map(|(i,s)| format!("{}. {}", i+1, s))
            .join("\n")
    }
}
