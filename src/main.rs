#![feature(plugin)]
#![plugin(regex_macros)]

extern crate itertools;
extern crate regex;
extern crate rustc_serialize;
extern crate strsim;
extern crate telegram_bot as telegram;

mod bot;
mod iter;
mod needed;

use bot::{MartiniBot, LogLevel};
use std::env;
use std::path::Path;

fn main() {
    // Fetch environment variable with bot token
    let token = match env::var("TELEGRAM_BOT_TOKEN") {
        Ok(tok) => tok,
        Err(e) =>
            panic!("Environment variable 'TELEGRAM_BOT_TOKEN' missing! {}", e),
    };

    // Create bot and print bot information, if it succeeded
    let mut bot = MartiniBot::from_token(token)
        .with_logfile(Path::new("log.txt"))
        .with_loglevel(LogLevel::Debug)
        .build()
        .unwrap_or_else(|e| panic!("Error starting bot: {} \n{:?}", e, e));

    let me = bot.me();
    bot.log(LogLevel::Info, &*format!("Started bot: {:?}", me));
    bot.run();
}
