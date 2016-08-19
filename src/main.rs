//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::str::FromStr;

extern crate clap;
use clap::{Arg, App};

#[macro_use]
extern crate log;
use log::LogLevelFilter;

extern crate hyper;

mod commands;
mod config;
mod logger;
mod webhooks;

include!("logger_macros.rs");

////////////////////////////////////////////////////////////
//                          Funcs                         //
////////////////////////////////////////////////////////////

//Utils
//NOTE: No logging here, these functions fire before logger is initialized
fn file_path_validator(file_path: String) -> Result<(), String> {
    match OpenOptions::new().read(true).write(true).create(true).open(Path::new(&file_path)) {
        Ok(_)    => Ok(()),
        Err(err) => {Err(format!("Cannot open file \"{}\" due to an error: \"{}\"", file_path.as_str(), err.description()))}
    }
}

fn dir_path_validator(dir_path: String) -> Result<(), String> {

    //Check if it's even a directory.
    match fs::metadata(&dir_path) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                let mut err_string = "\"".to_string();
                err_string.push_str(dir_path.as_str());
                err_string.push_str("\" is not a directory.");
                return Err(err_string);
            }
        },
        Err(err)     => {
            let mut err_string = "Failed to acquire metadata for \"".to_string();
            err_string.push_str(dir_path.as_str());
            err_string.push_str("\": ");
            err_string.push_str(err.description());
            return Err(err_string);
        }
    }

    //Test if we can write a file to this directory
    let mut counter = 0;
    loop {
        //Generate a file name.
        let mut tmp_file_path:     String = dir_path.clone();
        let     tmp_file_name:     String = format!("ab{}ba.tmp", counter);
        let     tmp_file_path_buf: PathBuf;
        tmp_file_path.push_str(tmp_file_name.as_str());
        tmp_file_path_buf = PathBuf::from(&tmp_file_path);

        //If file does not exist, check if we can create it with r/w permissions.
        if !tmp_file_path_buf.exists() {
            match OpenOptions::new().read(true).write(true).create(true).open(&tmp_file_path_buf.as_path()) {
                Ok(_)    => {
                    match fs::remove_file(&tmp_file_path_buf) {
                        Ok(_)    => return Ok(()),
                        Err(err) => {
                            let mut err_string = "Failed to delete temporary file: \"".to_string();
                            err_string.push_str(&tmp_file_path);
                            err_string.push_str("\": ");
                            err_string.push_str(err.description());
                            return Err(err_string);
                        }
                    }
                }
                Err(err) => {
                    let mut err_string = "Invalid directory: \"".to_string();
                    err_string.push_str(err.description());
                    err_string.push_str("\"");
                    return Err(err_string);
                }
            }
        }
        counter += 1;
    }
}

fn max_log_level_validator(max_log_level: String) -> Result<(), String> {
    match &max_log_level.to_lowercase()[..] {
        "error" => return Ok(()),
        "warn"  => return Ok(()),
        "info"  => return Ok(()),
        "debug" => return Ok(()),
        "trace" => return Ok(()),
        _       => return Err(String::from("Invalid log level"))
    }
}

fn log_size_validator(log_size: String) -> Result<(), String> {
    match u64::from_str(&log_size[..]) {
        Ok(_)    => return Ok(()),
        Err(err) => return Err(format!("Failed to parse the log size: {}", err.description()))
    }
}


////////////////////////////////////////////////////////////
//                          Main                          //
////////////////////////////////////////////////////////////

fn main() {

    let hunterbot_version = "0.1.0";
    let matches           = App::new("HunterBot")
    .version(hunterbot_version)
    .arg(Arg::with_name("CONFIG")
        .short("c")
        .long("config")
        .help("Sets a custom file path for the config.")
        .validator(file_path_validator)
        .takes_value(true))
    .arg(Arg::with_name("LOG")
        .short("l")
        .long("log-dir")
        .help("Sets a custom directory path for the log.")
        .validator(dir_path_validator)
        .takes_value(true))
    .arg(Arg::with_name("MAXLOGLEVEL")
        .short("m")
        .long("max-log-level")
        .help("Sets the maximum logging level, \"Info\" is the default, valid values are (in increasing order): error, warn, info, debug, trace.")
        .validator(max_log_level_validator)
        .takes_value(true))
    .arg(Arg::with_name("LOGSIZE")
        .short("s")
        .long("log-size")
        .help("Sets the maximum log file (in MB) before being rotated.")
        .validator(log_size_validator)
        .takes_value(true))
    .get_matches();

    let hunterbot_config_path = matches.value_of("CONFIG").unwrap_or("./HunterBotConfig.toml");
    let hunterbot_log_dir     = matches.value_of("LOG").unwrap_or("./");
    let log_size              = u64::from_str(matches.value_of("LOGSIZE").unwrap_or("5")).unwrap();
    let max_log_level         = match matches.value_of("MAXLOGLEVEL").unwrap_or("info") {
        "error" => LogLevelFilter::Error,
        "warn"  => LogLevelFilter::Warn,
        "info"  => LogLevelFilter::Info,
        "debug" => LogLevelFilter::Debug,
        "trace" => LogLevelFilter::Trace,
        _       => LogLevelFilter::Error
    };


    //Start logger
    let (logger, rx) = logger::Logger::new(&max_log_level);
    match logger::Logger::init(logger, max_log_level) {
        Ok(())   => (),
        Err(err) => {panic!("Failed to initialize the logger: {}", err);}
    }
    logger::Logger::process_logs(rx, PathBuf::from(hunterbot_log_dir), log_size);

    thread_info!("Logger booted.");
    thread_debug!("matches: {:?}", matches);
    thread_debug!("Option: hunterbot_config_path: {}", hunterbot_config_path);
    thread_debug!("Option: hunterbot_log_dir:     {}", hunterbot_log_dir);
    thread_debug!("Option: log_size:              {}", log_size);
    thread_debug!("Option: max_log_level:         {}", max_log_level);

    //Load config
    let mut config = config::ConfigHandler::new();

    thread_info!("Opening config...");
    match config.load(&hunterbot_config_path.to_string()) {
        Ok(())   => {thread_info!("Success!");}
        Err(err) => {thread_crash!("Error loading the config: {}", err);}
    }

    config.validate();

    //Setup webhooks
    webhooks::register(&mut config);

    //Listen for/process webhooks
    webhooks::listen(&mut config);
}
