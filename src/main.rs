extern crate telegram_bot as telegram;
extern crate rustc_serialize;

mod bot;
mod store;

// use rustc_serialize::json;
use std::env;

fn main() {
    // Fetch environment variable with bot token
    let token = match env::var("TELEGRAM_BOT_TOKEN") {
        Ok(tok) => tok,
        Err(e) =>
            panic!("Environment variable 'TELEGRAM_BOT_TOKEN' missing! {}", e),
    };

    // Create bot and print bot information, if it succeeded
    let mut bot = match bot::Martini::new(token) {
        Ok(bot) => {
            println!("Started bot: {:?}", bot.me());
            bot
        },
        Err(e) => panic!("Error starting bot: {:?}", e),
    };
    bot.run();

}
