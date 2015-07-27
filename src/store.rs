use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use telegram;

pub type FlatMap = HashMap<telegram::Integer, Arc<Mutex<FlatShare>>>;

#[derive(Default)]
pub struct FlatShare {
    pub needed: Vec<String>,
}
