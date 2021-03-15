use std::fs::File;
use std::io;
use std::path::Path;
use std::{borrow::Borrow, hash, io::Read};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AssettoMod {
    pub checksum_md5: String,
    pub filename: String,
    pub size_in_bytes: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub login: String,
    pub password_hash_sha512: String,
    pub is_admin: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub mods: Vec<AssettoMod>,
    pub mod_storage_location: String,
    pub server_paths: Vec<String>,
    pub users: Vec<User>,
}

pub trait ConfigTrait {
    fn new(path: &str) -> Self;
    fn add_user(
        &mut self,
        login: &String,
        new_password: &String,
        is_admin: bool,
    ) -> Result<(), &str>;
    fn change_user_password(&mut self, login: &String, new_password: &String) -> Result<(), &str>;
    fn delete_mod(&mut self, checksum_md5: &String) -> Result<(), String>;
    fn delete_user(&mut self, login: &String) -> Result<(), &str>;
    fn get_mod_list(&self) -> Vec<AssettoMod>;
    fn get_server_paths(&self) -> Vec<String>;
    fn is_login_data_valid(&self, login: &String, password: &String) -> bool;
    fn is_user_admin(&self, login: &String) -> bool;
    fn rebuild_mod_storage(&mut self, clear: bool) -> Result<(), String>;
    fn user_exists(&self, login: &String) -> bool;
}

pub struct ConfigObject {
    pub config: Config,
    pub path: String,
}

fn get_assetto_mod(path: &Path) -> Result<AssettoMod, String> {
    let input = File::open(path);
    if let Err(error) = input {
        return Err(error.to_string());
    }
    let mut file = input.unwrap();

    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    let bytes_processed = io::copy(&mut file, &mut hasher);
    if let Err(error) = bytes_processed {
        return Err(error.to_string());
    }
    let hash = hasher.finalize();

    Ok(AssettoMod {
        checksum_md5: format!("{:x}", hash),
        filename: path.file_name().unwrap().to_str().unwrap().to_string(),
        size_in_bytes: bytes_processed.unwrap(),
    })
}

fn write_config_to_json(path: &Path, config: &Config) {
    let json = serde_json::to_string_pretty(config);
    if let Ok(output) = json {
        std::fs::write(path, output).expect("Config file writing failure");
    }
}

fn calculate_sha512(value: &String) -> String {
    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::new();
    hasher.update(value);
    let hash = hasher.finalize();
    return format!("{:x}", hash);
}

impl ConfigTrait for ConfigObject {
    fn new(path: &str) -> ConfigObject {
        let file = std::fs::read_to_string(path);
        let content = file.unwrap();

        ConfigObject {
            config: serde_json::from_str(content.as_ref()).unwrap(),
            path: path.to_string(),
        }
    }

    fn add_user(
        &mut self,
        login: &String,
        new_password: &String,
        is_admin: bool,
    ) -> Result<(), &str> {
        if self.user_exists(login) {
            return Err("User already exists");
        }

        self.config.users.push(User {
            login: login.clone(),
            password_hash_sha512: calculate_sha512(new_password),
            is_admin: is_admin,
        });

        write_config_to_json(Path::new(&self.path), &self.config);

        Ok(())
    }

    fn change_user_password(&mut self, login: &String, new_password: &String) -> Result<(), &str> {
        for user in self.config.users.iter_mut() {
            if *login == user.login {
                use sha2::{Digest, Sha512};
                let mut hasher = Sha512::new();
                hasher.update(new_password);
                let hash = hasher.finalize();

                user.password_hash_sha512 = format!("{:x}", hash);
                write_config_to_json(Path::new(&self.path), &self.config);

                return Ok(());
            }
        }
        Err("User not found")
    }

    fn delete_mod(&mut self, checksum_md5: &String) -> Result<(), String> {
        let index = self
            .config
            .mods
            .iter()
            .position(|acmod| acmod.checksum_md5 == *checksum_md5);
        if let None = index {
            return Err("Mod not found".to_string());
        }
        let index = index.unwrap();

        let storage_path = Path::new(&self.config.mod_storage_location);
        let acmod = &self.config.mods[index];

        if let Err(error) = std::fs::remove_file(storage_path.join(&acmod.filename)) {
            return Err(error.to_string());
        }
        self.config.mods.remove(index);
        write_config_to_json(Path::new(&self.path), &self.config);
        Ok(())
    }

    fn delete_user(&mut self, login: &String) -> Result<(), &str> {
        self.config.users.retain(|user| user.login != *login);
        Ok(())
    }

    fn get_mod_list(&self) -> Vec<AssettoMod> {
        return self.config.mods.clone();
    }

    fn get_server_paths(&self) -> Vec<String> {
        return self.config.server_paths.clone();
    }

    fn is_login_data_valid(&self, login: &String, password: &String) -> bool {
        for user in self.config.users.iter() {
            if *login == user.login {
                return calculate_sha512(password) == user.password_hash_sha512;
            }
        }
        false
    }

    fn is_user_admin(&self, login: &String) -> bool {
        for user in self.config.users.iter() {
            if *login == user.login {
                return user.is_admin;
            }
        }
        false
    }

    fn rebuild_mod_storage(&mut self, clear: bool) -> Result<(), String> {
        let _ = std::fs::create_dir_all(&self.config.mod_storage_location);
        let paths = std::fs::read_dir(&self.config.mod_storage_location);
        if let Err(error) = paths {
            return Err(error.to_string());
        }

        if clear {
            self.config.mods.clear();
        }
        let mut mod_list: Vec<AssettoMod> = self.config.mods.clone();

        for path in paths.unwrap() {
            if let Err(error) = path {
                return Err(error.to_string());
            }
            let mod_path = path.unwrap().path();
            let exists = mod_list
                .iter()
                .any(|acmod| acmod.filename == mod_path.file_name().unwrap().to_str().unwrap());

            if exists {
                continue;
            }

            let assetto_mod = get_assetto_mod(mod_path.as_path())?;
            mod_list.push(assetto_mod);
        }

        self.config.mods = mod_list;

        write_config_to_json(Path::new(&self.path), &self.config);

        Ok(())
    }

    fn user_exists(&self, login: &String) -> bool {
        self.config.users.iter().any(|user| user.login == *login)
    }
}
