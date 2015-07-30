use std::fs;
use std::io;
use std::cmp::{Ordering, PartialOrd};
use std::path::Path;
use needed::Needed;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use telegram;
use std::default::Default;
use std::io::{Read, Write};
use rustc_serialize::json;


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
#[derive(Default, RustcEncodable, RustcDecodable)]
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
    data_dir: String,
}

pub struct BotBuilder<'a> {
    token: String,
    logfile: Option<&'a Path>,
    loglevel: LogLevel,
    data_dir: String,
}

/// Type for building a bot easily
#[allow(dead_code)]
impl<'a> BotBuilder<'a> {
    /// Specifies a logfile to log into. By default the bot does not log
    /// into a file.
    pub fn with_logfile<'b>(self, path: &'b Path) -> BotBuilder<'b> {
        BotBuilder{
            token: self.token,
            logfile: Some(path),
            loglevel: self.loglevel,
            data_dir: self.data_dir,
        }
    }

    /// Specifies the log level of the bot. All messages with level higher or
    /// equal to the specified level will be logged.
    pub fn with_loglevel(mut self, lvl: LogLevel) -> Self {
        self.loglevel = lvl;
        self
    }

    /// Specify the directory where the flatshare data lives.
    pub fn with_data_dir(mut self, dir: String) -> Self {
        self.data_dir = dir;
        self
    }

    /// Create a bot out of the given configuration.
    pub fn build(self) -> telegram::Result<MartiniBot> {
        // Create and test the api.
        let mut api = telegram::Bot::new(self.token);
        let me = try!(api.get_me());

        // Try to open the logfile in writing-append mode if specified
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

        // Check if the data directory exists. Create it otherwise.
        if let Err(e) = fs::create_dir(&Path::new(&*self.data_dir)) {
            if e.kind() != io::ErrorKind::AlreadyExists {
                return Err(e.into());
            }
        }

        Ok(MartiniBot {
            api: Arc::new(Mutex::new(api)),
            me: me,
            flats: FlatMap::new(),
            logger: Logger {
                logfile: file,
                loglevel: self.loglevel,
            },
            data_dir: self.data_dir,
        })
    }
}

impl MartiniBot {
    pub fn from_token(token: String) -> BotBuilder<'static> {
        BotBuilder {
            token: token,
            logfile: None,
            loglevel: LogLevel::Info,
            data_dir: "data/".into(),
        }
    }

    pub fn me(&self) -> telegram::User {
        self.me.clone()
    }

    pub fn log(&mut self, lvl: LogLevel, msg: &str) {
        self.logger.log(lvl, msg);
    }

    fn write_flat(&mut self, cid: telegram::Integer) {
        let fname = format!("{}{}.json", self.data_dir, cid);
        let p = &Path::new(&*fname);

        let file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(p);

        let mut file = match file {
            Ok(f) => f,
            Err(e) => {
                log!(self, Error: "Could not open file (w) '{}': {}", fname, e);
                return;
            }
        };

        let res = write!(file, "{}\n", json::as_pretty_json(self.flat(cid)));
        match res {
            Ok(_) =>
                log!(self, Debug: "Wrote file '{}'", fname),
            Err(e) =>
                log!(self, Warning: "Could not write file '{}': {}", fname, e),
        }
    }

    fn read_flat(&mut self, cid: telegram::Integer) -> Option<FlatShare> {
        let fname = format!("{}{}.json", self.data_dir, cid);
        let p = &Path::new(&*fname);

        let file = fs::File::open(p);

        let mut file = match file {
            Ok(f) => f,
            Err(e) => {
                log!(self, Debug: "Could not open file (r) '{}': {}", fname, e);
                return None;
            }
        };

        let mut content = String::new();
        if let Err(e) = file.read_to_string(&mut content) {
            log!(self, Warning: "Could not read file '{}': {}", fname, e);
            return None;
        }

        match json::decode(&*content) {
            Ok(f) => {
                log!(self, Debug: "Read file '{}'", fname);
                Some(f)
            },
            Err(e) => {
                log!(self, Warning: "Could not decode file '{}': {}", fname, e);
                None
            }
        }
    }

    fn flat(&mut self, cid: telegram::Integer) -> &mut FlatShare {
        if !self.flats.contains_key(&cid) {
            let new = self.read_flat(cid).unwrap_or(FlatShare::default());
            self.flats.insert(cid, new);
        }
        self.flats.get_mut(&cid).unwrap()
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

        // If the received update contains no text message: Return.
        let m = match u.message {
            Some(m) => m,
            None => return Ok(()),
        };
        let t = match m.msg {
            MessageType::Text(t) => t,
            _ => return Ok(()),
        };

        let name = if let Some(ln) = m.from.last_name {
            format!("{} {}", m.from.first_name, ln)
        } else {
            m.from.first_name
        };

        let cid = m.chat.id();

        // Match message type
        // Print received text message to stdout
        let room = match m.chat {
            Chat::Group(ref g) => format!("{}#'{}'", g.id, g.title),
            Chat::User(_) => "private".into(),
        };
        log!(self, Debug: "<{} @ {}> {}", name, room, t);

        let command = Regex::new(r"^/\w+").unwrap();
        // if let Some(com) = command.find(&*t).map(|(l,h)| t[l..h]) {
        //     // try!()
        // }

        if t.starts_with(&command) {
            let arg = t.trim_left_matches(&command);
            match t.splitn(2, " ").next().unwrap() {
                "/need" => {
                    let msg = self.flat(cid).needed.handle_need(arg.into());
                    try!(api.send_message(cid, msg, None, None, None));
                },
                "/got" => {
                    let msg = self.flat(cid).needed.handle_got(arg.into());
                    try!(api.send_message(cid, msg, None, None, None));
                }
                command => {
                    log!(self, Warning: "Unknown command '{}'", command);
                }
            }

        }

        // Update file
        if self.flats.contains_key(&cid) {
            self.write_flat(cid);
        }

        Ok(())
    }
}
