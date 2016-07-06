//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::sync::mpsc::channel;
use std::thread;
use std::process::exit;

mod config;

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


}
