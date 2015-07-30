use std::cmp::{Ord, Ordering};
use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use iter::LangIter;
use itertools::Itertools;
use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};



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

impl Deref for SimString {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.orig
    }
}

impl Display for SimString {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        self.orig.fmt(f)
    }
}

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

#[derive(Default, RustcEncodable, RustcDecodable)]
pub struct Needed {
    pub list: BTreeSet<SimString>,
}

impl Needed {
    // pub fn new() -> Needed {
    //     Needed { list: BTreeSet::new() }
    // }

    pub fn handle_need(&mut self, args: String) -> String {
        // Collect items that were already on the list.
        let mut already_there = Vec::new();

        // If the item was already on list: Report. Push to list otherwise.
        for item in args.split(',').map(|a| a.trim().to_owned().into()) {
            if self.list.contains(&item) {
                already_there.push(item);
            } else {
                self.list.insert(item);
            }
        }

        let mut msg = String::new();

        if !already_there.is_empty() {
            let list = already_there.iter()
                .map(|s| format!("'{}'", s))
                .listify(", ", " and ");
            msg = format!("{} already on the list!\n", list);
        }

        msg.push_str(&*format!("We need:\n{}", self.list.iter()
            .enumerate()
            .map(|(i,s)| format!("{}. {}", i+1, s))
            .join("\n")));

        msg
    }

    pub fn handle_got(&mut self, args: String) -> String {
        // Collect items that were not on the list.
        let mut not_found = Vec::new();


        for item in args.split(',').map(|a| a.trim().to_owned().into()) {
            if self.list.contains(&item) {
                self.list.remove(&item);
            } else {
                not_found.push(item);
            }
        }

        let mut msg = String::new();

        if !not_found.is_empty() {
            let list = not_found.iter()
                .map(|s| format!("'{}'", s))
                .join(", ");
            msg = format!("{} not on the list!\n", list);
        }

        if self.list.is_empty() {
            msg.push_str("We have everything we need :-)");
        } else {
            msg.push_str(&*format!("We still need:\n{}", self.list.iter()
                .enumerate()
                .map(|(i,s)| format!("{}. {}", i+1, s))
                .join("\n")));
        }

        msg
    }
}
