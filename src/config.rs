//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::error::Error;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

extern crate toml;

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

        trace!("config.rs: ConfigHandler::save()");

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
                trace!("      Create key and insert empty string");
                section_data.insert(key.to_string(), toml::Value::String(String::new()));
                self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
                trace!("Return Ok");
                return Ok(String::new());
            }
        } else {
            trace!("    false");
            trace!("    Create section and insert empty string as value for key");
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::String(String::new()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
            trace!("Return Ok");
            return Ok(String::new());
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
                trace!("      Insert empty table");
                section_data.insert(key.to_string(), toml::Value::Array(toml::Array::new()));
                self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
                trace!("Return Ok");
                return Ok(Vec::new());
            }
        } else {

            trace!("    false");
            trace!("    Insert section and empty table");
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::Array(toml::Array::new()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
            trace!("Return Ok");
            return Ok(Vec::new());
        }
    }

    //Is the user in the whitelist?
    //TODO: return an error on get failure
    pub fn whitelist_validate_user(&mut self, user: String) -> bool {

        trace!("config.rs: ConfigHandler::whitelist_validate_user(&mut self, \"{}\")", user);

        //Repo owner is always whitelisted
        trace!("  Get repo owner from config");
        match ConfigHandler::get_string(self, "config", "github_owner_name") {
            Ok(owner_name) => {
                trace!("    Is user the repo owner test");
                if owner_name == user {
                    trace!("Return true");
                    return true;
                }
            }
            Err(_)         => {
                //NOTE: This could be omitted once config validation is done.
                trace!("Return false (error getting the config)");
                return false
            }
        }

        let whitelist: Vec<toml::Value>;
        match self.get_array("config", "whitelist") {
            Ok(_whitelist) => whitelist = _whitelist,
            Err(err)       => {panic!("Error while getting the whitelist: {}", err);}
        }

        trace!("whitelist.contains(\"{}\")", user);
        let is_valid_user = whitelist.contains(&toml::Value::String(user.clone()));

        debug!("User {} is whitelisted: {}", user, is_valid_user);
        trace!("Return {}", is_valid_user);
        return is_valid_user;
    }
}