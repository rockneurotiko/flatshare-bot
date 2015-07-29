use std::fs;
use std::cmp::{Ordering, PartialOrd};
use std::path::Path;
use needed::Needed;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use telegram;
use std::default::Default;
use std::io::Write;


#[derive(PartialEq, Eq, Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn prefix(&self) -> &str {
        use self::LogLevel::*;
        match *self {
            Debug => "DEBUG",
            Info => "INFO ",
            Warning => "WARN ",
            Error => "ERROR",
        }
    }

    fn as_num(&self) -> u8 {
        use self::LogLevel::*;
        match *self {
            Debug   =>  0,
            Info    => 10,
            Warning => 20,
            Error   => 30,
        }
    }
}

impl PartialOrd for LogLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_num().partial_cmp(&other.as_num())
    }
}

// Data per FlatShare
#[derive(Default)]
struct FlatShare {
    needed: Needed,
}

// Maps a Telegram 'ChatID' to the corresponding flatshare data
pub type FlatMap = HashMap<telegram::Integer, Arc<Mutex<FlatShare>>>;

pub struct MartiniBot {
    api: telegram::Bot,
    me: telegram::User,
    flats: Arc<Mutex<FlatMap>>,
    logfile: Option<fs::File>,
    loglevel: LogLevel,
}

pub struct BotBuilder<'a> {
    token: String,
    logfile: Option<&'a Path>,
    loglevel: LogLevel,
}

impl<'a> BotBuilder<'a> {
    pub fn with_logfile<'b>(self, path: &'b Path) -> BotBuilder<'b> {
        BotBuilder{
            token: self.token,
            logfile: Some(path),
            loglevel: self.loglevel,
        }
    }

    pub fn with_loglevel(mut self, lvl: LogLevel) -> Self {
        self.loglevel = lvl;
        self
    }

    pub fn build(self) -> telegram::Result<MartiniBot> {
        let mut api = telegram::Bot::new(self.token);
        let me = try!(api.get_me());

        let file = match self.logfile {
            Some(path) => {
                Some(try!(fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(path)))
            },
            None => None,
        };

        Ok(MartiniBot {
            api: api,
            me: me,
            flats: Arc::new(Mutex::new(FlatMap::new())),
            logfile: file,
            loglevel: self.loglevel,
        })
    }
}

impl MartiniBot {

    pub fn from_token(token: String) -> BotBuilder<'static> {
        BotBuilder {
            token: token,
            logfile: None,
            loglevel: LogLevel::Info,
        }
    }

    pub fn me(&self) -> telegram::User {
        self.me.clone()
    }

    pub fn log(&mut self, lvl: LogLevel, msg: &str) {
        if lvl < self.loglevel {
            return;
        }

        if let Some(ref mut file) = self.logfile {
            if let Err(e) = write!(file, "[{}] {}", lvl.prefix(), msg) {
                panic!("Error occured while writing log file: {}\n{:?}", e, e);
            }
        }

        println!("[{}] {}", lvl.prefix(), msg);
    }

    pub fn run(&mut self) {
        // Fetch new updates via long poll method
        let flats = self.flats.clone();
        let res = self.api.long_poll(None, |api, u| {
            Self::handle(flats.clone(), api, u)
        });
        if let Err(e) = res {
            self.log(LogLevel::Error, &*format!("An error occured: {}", e));
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

            let flat_data = {
                let mut flats = flats.lock().unwrap();
                if !flats.contains_key(&chat_id) {
                    flats.insert(chat_id, Arc::new(Mutex::new(FlatShare::default())));
                }
                flats.get_mut(&chat_id).unwrap().clone()
            };
            let mut flat_data = flat_data.lock().unwrap();

            // Match message type
            if let MessageType::Text(t) = m.msg {
                // Print received text message to stdout
                // self.log(LogLevel::Debug, format!("<{}> {}", name, t));

                let command_prefix = Regex::new(r"/\w+ ").unwrap();
                if t.starts_with(&command_prefix) {
                    let arg = t.trim_left_matches(&command_prefix);
                    match t.splitn(2, " ").next().unwrap() {
                        "/need" => {
                            let msg = flat_data.needed.handle_need(arg.into());
                            try!(api.send_message(chat_id, msg, None, None, None));
                        },
                        "/got" => {
                            let msg = flat_data.needed.handle_got(arg.into());
                            try!(api.send_message(chat_id, msg, None, None, None));
                        }
                        command => {
                            // self.log(LogLevel::Warning, format!(
                            //     "Warning: Unknown command '{}'", command
                            // ));
                        }
                    }

                }
            }
        }
        Ok(())
    }

}
