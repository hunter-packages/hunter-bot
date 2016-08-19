//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};

extern crate chrono;
use self::chrono::*;

use log;
use log::{LogRecord, LogLevel, LogLevelFilter, LogMetadata, SetLoggerError};

include!("logger_macros.rs");

////////////////////////////////////////////////////////////
//                         Logger                         //
////////////////////////////////////////////////////////////

pub struct Logger{
    max_level: LogLevelFilter,
    log_tx:    Arc<Mutex<Sender<(String, LogLevel)>>>,
}

impl Logger {

    pub fn new(max_level: &LogLevelFilter) -> (Logger, Receiver<(String, LogLevel)>) {
        let (tx, rx) = channel::<(String, LogLevel)>();
        let logger = Logger {
            max_level: max_level.clone(),
            log_tx:    Arc::new(Mutex::new(tx)),
        };

        (logger, rx)
    }

    pub fn init(logger: Logger, max_level: LogLevelFilter) -> Result<(), SetLoggerError> {
        //Register backend
        log::set_logger(|max_log_level| {
            max_log_level.set(max_level);
            Box::new(logger)
        })
    }

    pub fn process_logs(rx: Receiver<(String, LogLevel)>, log_dir: PathBuf, log_size: u64) {
        thread::Builder::new().name(String::from("log")).spawn(move || {

            //Open log files
            let mut msg_log_file_name = get_next_logfile_path(&log_dir, "log-msg");
            let mut err_log_file_name = get_next_logfile_path(&log_dir, "log-err");
            let mut msg_log_file      = open_log_file(&msg_log_file_name);
            let mut err_log_file      = open_log_file(&err_log_file_name);

            loop {

                let (log, log_level) = rx.recv().unwrap();

                if log_level == LogLevel::Error {
                    if log == String::from("**INTERNAL** CRASH!!!") {exit(-1);}
                    err_log_file.write(log.as_bytes());
                } else {
                    msg_log_file.write(log.as_bytes());
                }

                //rotate if needed
                if needs_rotate(&msg_log_file_name, log_size) {
                    msg_log_file_name = get_next_logfile_path(&log_dir, "log-msg");
                    msg_log_file.write(format!("***INFO: Log has been rotated, further messages are in {}", msg_log_file_name).as_bytes());
                    msg_log_file      = open_log_file(&msg_log_file_name);
                }
                if needs_rotate(&err_log_file_name, log_size) {
                    err_log_file_name = get_next_logfile_path(&log_dir, "log-err");
                    err_log_file.write(format!("***INFO: Log has been rotated, further messages are in {}", msg_log_file_name).as_bytes());
                    err_log_file      = open_log_file(&err_log_file_name);
                }
            }
        });
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            let log_tx        = self.log_tx.clone();
            let time_and_date = Local::now().format("%v %H:%M:%S:%f").to_string();
            let internal      = format!("{}", record.args());
            let log_message   = format!("{}: {} {}\n", time_and_date, get_padded_loglevel_string(record.level().clone()), record.args());
            if internal == String::from("**INTERNAL** CRASH!!!") && record.level().clone() == LogLevel::Error {
                log_tx.lock().unwrap().send((internal, record.level()));
            } else {
                log_tx.lock().unwrap().send((log_message, record.level()));
            }
        }
    }
}


////////////////////////////////////////////////////////////
//                       Functions                        //
////////////////////////////////////////////////////////////

//Utils
pub fn get_padded_loglevel_string(log_level: LogLevel) -> String{
    match log_level {
        LogLevel::Error => return String::from("Error:"),
        LogLevel::Warn  => return String::from("Warn: "),
        LogLevel::Info  => return String::from("Info: "),
        LogLevel::Debug => return String::from("Debug:"),
        LogLevel::Trace => return String::from("Trace:")
    }
}

pub fn get_next_logfile_path(log_dir: &PathBuf, variant: &str) -> String {

    let mut counter = 0;

    loop {

        let     filename = format!("{}_{}_{}.log", variant, Local::now().format("%v").to_string(), counter);
        let mut path     = PathBuf::from(log_dir);
        path.push(filename.clone());
        match OpenOptions::new().write(true).create(false).open(path.clone()) {
            Ok(_)    => (), //File exists, lets keep looking for another one.
            Err(err) => {
                match err.kind() {
                    ErrorKind::NotFound         => {
                        match path.to_str() {
                            Some(file) => return String::from(file),
                            None       => {
                                thread_crash!("Failed to get file name from PathBuf (invalid UTF-8)");
                            }
                        }
                    },
                    ErrorKind::PermissionDenied => {thread_crash!("Cannot open {} to write logs into: Permission Denied.", path.display());}
                    _                           => {thread_crash!("Failed to open log file: {}", err);}
                }
            }
        }

        counter += 1;
    }
}

pub fn open_log_file(file_name: &String ) -> File {
    match OpenOptions::new().write(true).create(true).open(file_name) {
        Ok(file) => return file,
        Err(err) => {thread_crash!("Failed to crate log file: {}", err.description());}
    };
}

pub fn needs_rotate(file_name: &String, log_size: u64) -> bool{

    //get metadata
    let metadata = match fs::metadata(file_name) {
        Ok(metadata) => metadata,
        Err(err)     => {thread_crash!("Failed to acquire metadata of the current log file: {}", err.description());}
    };

    if metadata.len()/1000000 > log_size {
        return true;
    }

    false
}
