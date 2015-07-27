// use telegram::Bot;
use std::sync::{Mutex, Arc};
use telegram;
use store::{FlatMap, FlatShare};


pub struct Martini {
    api: telegram::Bot,
    me: telegram::User,
    flats: Arc<Mutex<FlatMap>>,
}

impl Martini {
    pub fn new(token: String) -> telegram::Result<Self> {
        let mut api = telegram::Bot::new(token);
        let me = try!(api.get_me());

        Ok(Martini {
            api: api,
            me: me,
            flats: Arc::new(Mutex::new(FlatMap::new())),
        })
    }

    pub fn me(&self) -> &telegram::User {
        &self.me
    }

    pub fn run(&mut self) {
        // Fetch new updates via long poll method
        let flats = self.flats.clone();
        let res = self.api.long_poll(None, |api, u| {
            Self::handle(flats.clone(), api, u)
        });
        if let Err(e) = res {
            println!("An error occured: {}", e);
        }
    }

    fn handle(flats: Arc<Mutex<FlatMap>>,
              api: &mut telegram::Bot,
              u: telegram::Update)
        -> telegram::Result<()>
    {
        use telegram::types::*;

        // If the received update contains a message...
        if let Some(m) = u.message {
            let name = m.from.first_name + &*m.from.last_name
                .map_or("".to_string(), |mut n| { n.insert(0, ' '); n });
            let chat_id = m.chat.id();

            let chat_data = {
                let mut flats = flats.lock().unwrap();
                if !flats.contains_key(&chat_id) {
                    flats.insert(chat_id, Arc::new(Mutex::new(FlatShare::default())));
                }
                flats.get_mut(&chat_id).unwrap().clone()
            };
            let mut chat_data = chat_data.lock().unwrap();

            // Match message type
            if let MessageType::Text(t) = m.msg {
                // Print received text message to stdout
                println!("<{}> {}", name, t);

                if t.starts_with("/weneed ") {
                    let mut needed = &mut chat_data.needed;
                    for item in t.trim_left_matches("/weneed ").split(',') {
                        needed.push(item.to_string());
                    }

                    let msg = needed.iter().enumerate()
                        .map(|(i,s)| format!("{}. {}\n", i+1, s))
                        .fold("We need:\n".to_string(), |acc, s| acc + &*s);

                    try!(api.send_message(chat_id, msg, None, None, None));
                }

                // Reply with custom Keyboard

            }

        }
        Ok(())
    }
}
