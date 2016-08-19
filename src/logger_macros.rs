//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::{thread, time};

extern crate thread_id;

////////////////////////////////////////////////////////////
//                        Functions                       //
////////////////////////////////////////////////////////////

pub fn hang() -> ! {
    let time = time::Duration::from_secs(1);
    loop{
        thread::sleep(time);
    }
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
        error!("Thread ID {}: {}", thread_id::get(), format_args!($($msg)*));
    }
}

macro_rules! thread_warn {
    ($($msg:tt)*) => {
        warn!("Thread ID {}: {}", thread_id::get(), format_args!($($msg)*));
    }
}

macro_rules! thread_info {
    ($($msg:tt)*) => {
        info!("Thread ID {}: {}", thread_id::get(), format_args!($($msg)*));
    }
}

macro_rules! thread_debug {
    ($($msg:tt)*) => {
        debug!("Thread ID {}: {}", thread_id::get(), format_args!($($msg)*));
    }
}

macro_rules! thread_trace {
    ($($msg:tt)*) => {
        trace!("Thread ID {}: {}", thread_id::get(), format_args!($($msg)*));
    }
}
