//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::process::exit;

extern crate hyper;

mod config;
mod webhooks;

//TODO: Output to log.

fn main() {

    let hunter_bot_config_path = "./HunterBotConfig.toml";

    //Load config
    let mut config = config::ConfigHandler::new();
    println!("Opening config...");

    match config.load(&hunter_bot_config_path.to_string()) {
        Ok(())   => {println!("Success!");}
        Err(err) => {
            println!("Error! {}", err);
            println!("Exiting...");
            exit(-1);
        }
    }

    //Setup webhooks
    webhooks::register(&mut config);

    //Listen for/process webhooks
    webhooks::listen(&mut config);
}
