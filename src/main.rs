//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::sync::mpsc::channel;
use std::thread;
use std::process::exit;

extern crate hyper;

mod config;
mod utilities;
mod webhooks;

//TODO: Output to log.

////////////////////////////////////////////////////////////
//                          Main                          //
////////////////////////////////////////////////////////////

fn main() {

    let hunter_bot_config_path = "./HunterBotConfig.toml";


    ////////////////////////////////////////////////////////////
    //                       Load Config                      //
    ////////////////////////////////////////////////////////////

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


    ////////////////////////////////////////////////////////////
    //                    Setup Webhooks                      //
    ////////////////////////////////////////////////////////////

    //setup
    webhooks::register(&mut config);
    //listen

}
