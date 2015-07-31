use std::cmp::{Ordering, PartialOrd};
use std::fs;
use std::io::Write;

/// Several levels of importance for log messages. Can be compared via the
/// standard `<`, `>`, ... comparison operators.
#[derive(PartialEq, Eq, Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    /// The five-character abbreviation for printing.
    pub fn prefix(&self) -> &str {
        use self::LogLevel::*;
        match *self {
            Debug => "DEBUG",
            Info => "INFO ",
            Warning => "WARN ",
            Error => "ERROR",
        }
    }

    /// Just some arbitrary numbers to make comparison easier.
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

/// Has some functionality to print messages to the terminal and write them
/// into a file.
pub struct Logger {
    pub logfile: Option<fs::File>,
    pub loglevel: LogLevel,
}

impl Logger {
    /// Post a log message with a given log level.
    pub fn log(&mut self, lvl: LogLevel, msg: &str) {
        use term_painter::{Attr, ToStyle};
        use term_painter::Color::*;
        use self::LogLevel::*;

        // If the level of the given message is lower than the set level, the
        // message is not processed.
        if lvl < self.loglevel {
            return;
        }

        // If logging to file was enabled, write the message to that file.
        if let Some(ref mut file) = self.logfile {
            if let Err(e) = write!(file, "[{}] {}\n", lvl.prefix(), msg) {
                panic!("Error occured while writing log file: {}\n{:?}", e, e);
            }
        }

        // Set colors for terminal output and print it.
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

/// Macro for easier use. This is used similar to `format!`. Example:
///
///     log!(self, Warning: "Evil people: {}", people);
macro_rules! log {
    ($this:ident, $lvl:ident: $fmt:expr) => {
        $this.logger.log(LogLevel::$lvl, $fmt);
    };
    ($this:ident, $lvl:ident: $fmt:expr, $($arg:tt)*) => {
        $this.logger.log(LogLevel::$lvl, &*format!($fmt, $($arg)*));
    };
}
