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

struct Logger {
    logfile: Option<fs::File>,
    loglevel: LogLevel,
}

impl Logger {
    pub fn log(&mut self, lvl: LogLevel, msg: &str) {
        use term_painter::{Attr, ToStyle};
        use term_painter::Color::*;
        use self::LogLevel::*;

        if lvl < self.loglevel {
            return;
        }

        if let Some(ref mut file) = self.logfile {
            if let Err(e) = write!(file, "[{}] {}\n", lvl.prefix(), msg) {
                panic!("Error occured while writing log file: {}\n{:?}", e, e);
            }
        }

        let prefix = match lvl {
            Error   => Attr::Bold.fg(Red),
            Warning => Attr::Bold.fg(Yellow),
            Info    => Attr::Plain.fg(White),
            Debug   => Attr::Dim.fg(NotSet),
        };
        let text = match lvl {
            Error   => Attr::Bold.fg(Red),
            Warning => Attr::Plain.fg(Yellow),
            Info    => Attr::Plain.fg(NotSet),
            Debug   => Attr::Plain.fg(NotSet),
        };

        println!("[{}] {}", prefix.paint(lvl.prefix()), text.paint(msg));
    }
}

macro_rules! log {
    ($this:ident, $lvl:ident: $fmt:expr) => {
        $this.logger.log(LogLevel::$lvl, $fmt);
    };
    ($this:ident, $lvl:ident: $fmt:expr, $($arg:tt)*) => {
        $this.logger.log(LogLevel::$lvl, &*format!($fmt, $($arg)*));
    };
}

// Data per FlatShare
#[derive(Default)]
struct FlatShare {
    needed: Needed,
}

// Maps a Telegram 'ChatID' to the corresponding flatshare data
pub type FlatMap = HashMap<telegram::Integer, FlatShare>;

pub struct MartiniBot {
    api: Arc<Mutex<telegram::Bot>>,
    me: telegram::User,
    flats: FlatMap,
    logger: Logger,
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
            api: Arc::new(Mutex::new(api)),
            me: me,
            flats: FlatMap::new(),
            logger: Logger {
                logfile: file,
                loglevel: self.loglevel,
            }
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
        self.logger.log(lvl, msg);
    }

    pub fn run(&mut self) {
        // Fetch new updates via long poll method
        let api = self.api.clone();
        let res = api.lock().unwrap().long_poll(None, |api, u| {
            self.handle(api, u)
        });
        if let Err(e) = res {
            log!(self, Error: "An error occured: {}", e);
        }
    }

    fn handle(&mut self,
              api: &mut telegram::Bot,
              u: telegram::Update)
        -> telegram::Result<()>
    {
        use telegram::types::*;

        // If the received update contains a message...
        if let Some(m) = u.message {
            let name = m.from.first_name + &*m.from.last_name
                .map_or("".to_string(), |mut n| { n.insert(0, ' '); n });
            let cid = m.chat.id();

            // if !self.flats.contains_key(&cid) {
            //     self.flats.insert(cid, FlatShare::default());
            // }
            // let flat = self.flats.get_mut(&cid).unwrap();
            let flat = self.flats.entry(cid);


            // Match message type
            if let MessageType::Text(t) = m.msg {
                // Print received text message to stdout
                log!(self, Debug: "<{}> {}", name, t);

                let command_prefix = Regex::new(r"/\w+ ").unwrap();
                if t.starts_with(&command_prefix) {
                    let arg = t.trim_left_matches(&command_prefix);
                    match t.splitn(2, " ").next().unwrap() {
                        "/need" => {
                            let msg = flat.or_insert(FlatShare::default()).needed.handle_need(arg.into());
                            try!(api.send_message(cid, msg, None, None, None));
                        },
                        "/got" => {
                            let msg = flat.or_insert(FlatShare::default()).needed.handle_got(arg.into());
                            try!(api.send_message(cid, msg, None, None, None));
                        }
                        command => {
                            log!(self, Warning: "Unknown command '{}'", command);
                        }
                    }

                }
            }
        }
        Ok(())
    }

}
