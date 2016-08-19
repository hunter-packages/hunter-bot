//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

extern crate toml;

include!("logger_macros.rs");

////////////////////////////////////////////////////////////
//                     ConfigHandler                      //
////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct ConfigHandler {
    pub toml_data: toml::Table,
    pub file_path: PathBuf
}

impl ConfigHandler {

    pub fn new() -> ConfigHandler {
        trace!("config.rs: ConfigHandler::new()");
        ConfigHandler {
            toml_data: toml::Table::new(),
            file_path: PathBuf::new()
        }
    }

    //Load config.
    pub fn load(&mut self, config_path: &String) -> Result<(), String> {

        trace!("config.rs: ConfigHandler::load(&mut self, \"{}\")", config_path);

        self.file_path = PathBuf::from(config_path);

        //Load file.
        trace!("  Load file");
        let mut file = match File::open(self.file_path.clone().as_path()) {
            Ok(file) => file,
            Err(err) => {
                trace!("    Return error");
                return Err(err.description().to_string())
            }
        };
        trace!("    Ok");

        //Get config file metadata.
        trace!("  Acquire metadata");
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(err)     => {
                trace!("    Return error");
                return Err(err.description().to_string())
            }
        };
        trace!("    Ok");

        //Parse config file.
        trace!("  Is file length 0 test");
        let file_length: usize = metadata.len() as usize;
        if file_length == 0 {
            trace!("    true");
            trace!("    New config");
            self.toml_data = toml::Table::new();
        } else {

            //Read file.
            trace!("    false");
            trace!("    Read file");
            let mut file_data: String = String::with_capacity(file_length +1);
            match file.read_to_string(&mut file_data) {
                Ok(_)    => (),
                Err(err) => {
                    trace!("      Return error");
                    return Err(err.description().to_string())
                }
            }
            trace!("      Ok");

            //Parse
            trace!("    Parse");
            self.toml_data = match toml::Parser::new(&file_data).parse().ok_or("Failed to parse toml data."){
                Ok(toml_data) => toml_data,
                Err(err)      => {
                    trace!("      Return error");
                    return Err(err.to_string())
                }
            };
            trace!("      Ok");
        }
        trace!("Return Ok");
        Ok(())
    }

    //Save config
    //TODO: fix oks
    pub fn save(&mut self) -> Result<(), String> {

        trace!("config.rs: ConfigHandler::save(&mut self)");

        let toml_data_string = toml::encode_str(&self.toml_data);

        //Open file, overwrite config with what we have
        trace!("  Open file");
        let mut file: File;
        match OpenOptions::new().write(true).truncate(true).open(self.file_path.clone().as_path()) {
            Ok(_file) => {
                trace!("    Ok");
                trace!("    Overwrite data");
                file = _file;
                match file.write_all(toml_data_string.as_bytes()) {
                    Ok(())   => {
                        trace!("      Ok");
                        ()
                    },
                    Err(err) => {
                        trace!("      Return error");
                        return Err(err.description().to_string())
                    }
                }
            }
            Err(err)  => {
                trace!("    Return error");
                return Err(err.description().to_string())
            }
        }
        trace!("Return Ok");
        Ok(())
    }

    //Validates config, checks for non empty values in required config keys
    //Would rather crash now than crash way later on
    pub fn validate(&mut self) {

        trace!("config.rs: ConfigHandler::validate(&mut self)");

        let github_bot_name = self.get_string_required("config", "github_bot_name");
        if github_bot_name == String::new() {
            crash!("Required config value of \"github_bot_name\" must be non-empty.");
        }

        let github_bot_token = self.get_string_required("config", "github_bot_token");
        if github_bot_token == String::new() {
            crash!("Required config value of \"github_bot_token\" must be non-empty.");
        }

        let github_follow_repo = self.get_string_required("config", "github_follow_repo");
        if github_follow_repo == String::new() {
            crash!("Required config value of \"github_follow_repo\" must be non-empty.");
        }

        let github_owner_name = self.get_string_required("config", "github_owner_name");
        if github_owner_name == String::new() {
            crash!("Required config value of \"github_owner_name\" must be non-empty.");
        }

        let github_owner_token = self.get_string_required("config", "github_owner_token");
        if github_owner_token == String::new() {
            crash!("Required config value of \"github_owner_token\" must be non-empty.");
        }

        let listen_port = self.get_string_required("config", "listen_port");
        if listen_port == String::new() {
            crash!("Required config value of \"listen_port\" must be non-empty.");
        }

        let local_ip_address = self.get_string_required("config", "local_ip_address");
        if local_ip_address == String::new() {
            crash!("Required config value of \"local_ip_address\" must be non-empty.");
        }

        let public_ip_address = self.get_string_required("config", "public_ip_address");
        if public_ip_address == String::new() {
            crash!("Required config value of \"public_ip_address\" must be non-empty.");
        }

        //Ok to be empty
        let whitelist = self.get_array_required("config", "whitelist");

        info!("Config validation passed.");
        debug!("Config value \"github_bot_name\" =    \"{}\"", github_bot_name);
        debug!("Config value \"github_bot_token\" =   \"{}\"", github_bot_token);
        debug!("Config value \"github_follow_repo\" = \"{}\"", github_follow_repo);
        debug!("Config value \"github_owner_name\" =  \"{}\"", github_owner_name);
        debug!("Config value \"github_owner_token\" = \"{}\"", github_owner_token);
        debug!("Config value \"listen_port\" =        \"{}\"", listen_port);
        debug!("Config value \"local_ip_address\" =   \"{}\"", local_ip_address);
        debug!("Config value \"public_ip_address\" =  \"{}\"", public_ip_address);
        debug!("Config value \"whitelist\" =          {:?}",  whitelist);

    }

    //Sets a config key-value pair
    pub fn set_string(&mut self, section: &str, key: &str, val: &str) {

        trace!("config.rs: ConfigHandler::set_string(&mut self, \"{}\", \"{}\", \"{}\")", section, key, val);
        trace!("  Does key exist test");
        if self.toml_data.contains_key(&section.to_string()) {

            //Get data from the section, insert key-value pair, reinsert section
            trace!("    true");
            trace!("    Get data, insert key-vaule pair, reinsert");
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();
            section_data.insert(key.to_string(), toml::Value::String(val.to_string()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
        } else {

            //Create section and insert
            trace!("    false");
            trace!("    Create section and insert");
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::String(val.to_string()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
        }
    }

    //Sets a config key-array pair
    pub fn set_array(&mut self, section: &str, key: &str, val: &toml::Array) {

        trace!("config.rs: ConfigHandler::set_array(&mut self, \"{}\", \"{}\", \"{:?}\")", section, key, val);
        trace!("  Does key exist test");
        if self.toml_data.contains_key(&section.to_string()) {

            //Get data from the section, insert key-value pair, reinsert section
            trace!("    true");
            trace!("    Get data");
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();
            section_data.insert(key.to_string(), toml::Value::Array(val.clone()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
        } else {

            //Create section and insert
            trace!("    false");
            trace!("    Create section and insert");
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::Array(val.clone()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
        }
    }

    //Gets a config value for a key, returns "" if key doesnt exist and creates the key
    pub fn get_string(&mut self, section: &str, key: &str) -> Result<String, String> {


        //Does the section exist? If not create it and insert empty string for given key.
        trace!("config.rs: ConfigHandler::get_string(&mut self, \"{}\", \"{}\")", section, key);
        trace!("  Does section exist test");
        if self.toml_data.contains_key(&section.to_string()) {

            trace!("    true");

            //Get data from the section
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();

            //Does the key/value pair exist? If not create it and insert empty string for given key.
            trace!("    Does key-value pair exists test");
            if section_data.contains_key(&key.to_string()) {

                trace!("      true");

                //Is it a string? If not return err.
                trace!("      Get data as string");
                match section_data.get(&key.to_string()).unwrap().clone().as_str().ok_or(format!("The \"{}\" field in \"[{}]\" of the config does not represent a string.", key, section)) {
                    Ok(key_value) => {
                        trace!("Return Ok");
                        return Ok(key_value.to_string())
                    },
                    Err(err)      => {
                        trace!("Return Error");
                        return Err(err)
                    }
                }
            } else {
                trace!("      false");
                trace!("Return Err");
                return Err(format!("Key \"{}\" in section \"[{}]\" does not exist.", key, section));
            }
        } else {
            trace!("    false");
            trace!("Return Err");
            return Err(format!("Section \"[{}]\" does not exist.", section));
        }
    }

    //Same as get_string() but crashes if key/value pair is missing as its required for the bot to function
    pub fn get_string_required(&mut self, section: &str, key: &str) -> String {
        thread_trace!("config.rs: ConfigHandler::get_string_required(&mut self, \"{}\", \"{}\")", section, key);
        match self.get_string(&section, &key) {
            Ok(value) => {
                thread_trace!("Return value");
                return value
            },
            Err(err)  => {
                thread_crash!("Error getting the value of \"{}\" from the config which is required: {}", key, err);
            }
        }
    }

    //Gets a config value array for a key, returns an empty toml::Array if key doesnt exist and creates the key
    pub fn get_array(&mut self, section: &str, key: &str) -> Result<Vec<toml::Value>, String> {

        trace!("config.rs: ConfigHandler::get_array(&mut self, \"{}\", \"{}\")", section, key);

        //Does the section exist? If not create it and insert empty string for given key.
        trace!("  Does section exist test");
        if self.toml_data.contains_key(&section.to_string()) {

            trace!("    true");

            //Get data from the section
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();

            //Does the key/value pair exist? If not create it and insert empty string for given key.
            trace!("    Does key-value pair exist test");
            if section_data.contains_key(&key.to_string()) {

                trace!("      true");

                //Is it an array? If not return err.
                trace!("      Get data as array");
                let mut array: Vec<toml::Value> = Vec::new();
                match section_data.get(&key.to_string()).unwrap().clone().as_slice().ok_or(format!("The \"{}\" field in \"[{}]\" of the config does not represent an array.", key, section)) {
                    Ok(key_value) => {
                        array.extend_from_slice(key_value);
                        trace!("Return Ok");
                        return Ok(array)
                    }
                    Err(err)      => {
                        trace!("Return Err");
                        return Err(err)
                    }
                }
            } else {

                trace!("      false");
                trace!("Return Err");
                return Err(format!("Key \"{}\" in section \"[{}]\" does not exist.", key, section));
            }
        } else {

            trace!("    false");
            trace!("Return Err");
            return Err(format!("Section \"[{}]\" does not exist.", section));
        }
    }

    //Same as get_array() but crashes if key/value pair is missing as its required for the bot to function
    pub fn get_array_required(&mut self, section: &str, key: &str) -> Vec<toml::Value> {
        thread_trace!("config.rs: ConfigHandler::get_array_required(&mut self, \"{}\", \"{}\")", section, key);
        match self.get_array(&section, &key) {
            Ok(value) => {
                thread_trace!("Return value");
                return value
            },
            Err(err)  => {
                thread_crash!("Error getting the value of \"{}\" from the config which is required: {}", key, err);
            }
        }
    }

    //Is the user in the whitelist?
    pub fn whitelist_validate_user(&mut self, user: String) -> bool {

        thread_trace!("config.rs: ConfigHandler::whitelist_validate_user(&mut self, \"{}\")", user);

        //Repo owner is always whitelisted
        let owner_name = self.get_string_required("config", "github_owner_name");
        thread_trace!("  Is user the repo owner test");
        if owner_name == user {
            thread_trace!("Return true");
            return true;
        }

        let whitelist = self.get_array_required("config", "whitelist");

        thread_trace!("whitelist.contains(\"{}\")", user);
        let is_valid_user = whitelist.contains(&toml::Value::String(user.clone()));

        thread_debug!("User {} is whitelisted: {}", user, is_valid_user);
        thread_trace!("Return {}", is_valid_user);
        return is_valid_user;
    }
}
