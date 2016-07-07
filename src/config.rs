//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::error::Error;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

extern crate toml;

////////////////////////////////////////////////////////////
//                        Structs                         //
////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct ConfigHandler {
    pub toml_data: toml::Table,
    pub file_path: PathBuf
}


////////////////////////////////////////////////////////////
//                         Impls                          //
////////////////////////////////////////////////////////////

impl ConfigHandler {

    pub fn new() -> ConfigHandler {
        ConfigHandler {
            toml_data: toml::Table::new(),
            file_path: PathBuf::new()
        }
    }

    //Load config.
    pub fn load(&mut self, config_path: &String) -> Result<(), String> {

        self.file_path = PathBuf::from(config_path);

        //Load file.
        let mut file: File;
        match File::open(self.file_path.clone().as_path()) {
            Ok(_file) => file = _file,
            Err(err)  => return Err(err.description().to_string())
        }

        //Get config file metadata.
        let metadata: Metadata;
        match file.metadata() {
            Ok(_metadata) => metadata = _metadata,
            Err(err)      => return Err(err.description().to_string())
        }

        //Parse config file.
        let file_length: usize = metadata.len() as usize;
        if file_length == 0 {
            self.toml_data = toml::Table::new();
        } else {

            //Read file.
            let mut file_data: String = String::with_capacity(file_length +1);
            match file.read_to_string(&mut file_data) {
                Ok(_)    => (),
                Err(err) => return Err(err.description().to_string())
            }

            //Parse
            self.toml_data = match toml::Parser::new(&file_data).parse().ok_or("Failed to parse toml data."){
                Ok(_toml_data) => _toml_data,
                Err(err)       => return Err(err.to_string())
            }
        }
        Ok(())
    }

    //Save config
    pub fn save(&mut self) -> Result<(), String> {

        let toml_data_string = toml::encode_str(&self.toml_data);

        //Open file, overwrite config with what we have
        let mut file: File;
        match OpenOptions::new().write(true).truncate(true).open(self.file_path.clone().as_path()) {
            Ok(_file) => {
                file = _file;
                match file.write_all(toml_data_string.as_bytes()) {
                    Ok(())   => (),
                    Err(err) => return Err(err.description().to_string())
                }
            }
            Err(err)  => return Err(err.description().to_string())
        }
        Ok(())
    }

    //Sets a config key-value pair
    pub fn set_string(&mut self, section: &str, key: &str, val: &str) {
        if self.toml_data.contains_key(&section.to_string()) {
            //Get data from the section, insert key-value pair, reinsert section
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();
            section_data.insert(key.to_string(), toml::Value::String(val.to_string()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
        } else {
            //Create section and insert
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::String(val.to_string()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
        }
    }

    //Sets a config key-array pair
    pub fn set_array(&mut self, section: &str, key: &str, val: &toml::Array) {
        if self.toml_data.contains_key(&section.to_string()) {
            //Get data from the section, insert key-value pair, reinsert section
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();
            section_data.insert(key.to_string(), toml::Value::Array(val.clone()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
        } else {
            //Create section and insert
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::Array(val.clone()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
        }
    }

    //Gets a config value for a key, returns "" if key doesnt exist and creates the key
    pub fn get_string(&mut self, section: &str, key: &str) -> Result<String, &'static str> {

        //Does the section exist? If not create it and insert empty string for given key.
        if self.toml_data.contains_key(&section.to_string()) {

            //Get data from the section
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();

            //Does the key/value pair exist? If not create it and insert empty string for given key.
            if section_data.contains_key(&key.to_string()) {

                //Is it a string? If not return err.
                match section_data.get(&key.to_string()).unwrap().clone().as_str().ok_or("The requested value is not a string.") {
                    Ok(key_value) => return Ok(key_value.to_string()),
                    Err(err)      => return Err(err)
                }
            } else {
                section_data.insert(key.to_string(), toml::Value::String(String::new()));
                self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
                return Ok(String::new());
            }
        } else {
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::String(String::new()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
            return Ok(String::new());
        }
    }

    //Gets a config value array for a key, returns an empty toml::Array if key doesnt exist and creates the key
    pub fn get_array(&mut self, section: &str, key: &str) -> Result<Vec<toml::Value>, &'static str> {

        //Does the section exist? If not create it and insert empty string for given key.
        if self.toml_data.contains_key(&section.to_string()) {

            //Get data from the section
            let mut section_data = self.toml_data.get(&section.to_string()).unwrap().as_table().unwrap().clone();

            //Does the key/value pair exist? If not create it and insert empty string for given key.
            if section_data.contains_key(&key.to_string()) {

                //Is it an array? If not return err.
                let mut array: Vec<toml::Value> = Vec::new();
                match section_data.get(&key.to_string()).unwrap().clone().as_slice().ok_or("The requested value is not an array.") {
                    Ok(key_value) => {
                        array.extend_from_slice(key_value);
                        return Ok(array)
                    }
                    Err(err)      => return Err(err)
                }
            } else {
                section_data.insert(key.to_string(), toml::Value::Array(toml::Array::new()));
                self.toml_data.insert(section.to_string(), toml::Value::Table(section_data));
                return Ok(Vec::new());
            }
        } else {
            let mut table = toml::Table::new();
            table.insert(key.to_string(), toml::Value::Array(toml::Array::new()));
            self.toml_data.insert(section.to_string(), toml::Value::Table(table));
            return Ok(Vec::new());
        }
    }

    //Is the user in the whitelist?
    pub fn whitelist_validate_user(&mut self, user: String) -> bool {

        //Repo owner is always whitelisted
        match ConfigHandler::get_string(self, "config", "github_owner_name") {
            Ok(owner_name) => {
                if owner_name == user {
                    return true;
                }
            }
            Err(_)         => return false
        }

        let whitelist: Vec<toml::Value>;
        match self.get_array("config", "whitelist") {
            Ok(_whitelist) => whitelist = _whitelist,
            Err(err)       => {panic!("Error while getting the whitelist: {}", err);}
        }
        return whitelist.contains(&toml::Value::String(user));
    }
}