//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::thread;
use std::time;


////////////////////////////////////////////////////////////
//                        Functions                       //
////////////////////////////////////////////////////////////

pub fn hang() -> ! {
    let time = time::Duration::from_secs(1);
    loop{
        thread::sleep(time);
    }
}

pub fn thread_name() -> String {
    let mut thread_name = thread::current().name().unwrap_or("<unknown>").to_string();
    thread_name         = thread_name.replace("<", "");
    thread_name         = thread_name.replace(">", "");
    thread_name
}

////////////////////////////////////////////////////////////
//                         Macros                         //
////////////////////////////////////////////////////////////

macro_rules! crash {
    ($($msg:tt)*) => {
        error!("**CRASH**: {}", format_args!($($msg)*));
        println!("**CRASH**: {}", format_args!($($msg)*));
        //Lets the remaining logs write to file before terminating the program.
        error!("**INTERNAL** CRASH!!!");
        //Hang until the program gets terminated
        hang();
    }
}

macro_rules! thread_crash {
    ($($msg:tt)*) => {
        thread_error!("**CRASH**: {}", format_args!($($msg)*));
        println!("**CRASH**: {}", format_args!($($msg)*));
        //Lets the remaining logs write to file before terminating the program.
        error!("**INTERNAL** CRASH!!!");
        //Hang until the program gets terminated
        hang();
    }
}

macro_rules! thread_error {
    ($($msg:tt)*) => {
        error!("Thread '{}': {}", thread_name(), format_args!($($msg)*));
    }
}

macro_rules! thread_warn {
    ($($msg:tt)*) => {
        warn!("Thread '{}': {}", thread_name(), format_args!($($msg)*));
    }
}

macro_rules! thread_info {
    ($($msg:tt)*) => {
        info!("Thread '{}': {}", thread_name(), format_args!($($msg)*));
    }
}

macro_rules! thread_debug {
    ($($msg:tt)*) => {
        debug!("Thread '{}': {}", thread_name(), format_args!($($msg)*));
    }
}

macro_rules! thread_trace {
    ($($msg:tt)*) => {
        trace!("Thread '{}': {}", thread_name(), format_args!($($msg)*));
    }
}
