use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use fs_extra::dir::CopyOptions;
use rocket::http::{ContentType, Cookies};
use rocket::request::{FlashMessage, Request};
use rocket::response::{Flash, Redirect};
use rocket::{Data, State};
use rocket_contrib::templates::Template;
use rocket_multipart_form_data::{
    MultipartFormData, MultipartFormDataField, MultipartFormDataOptions, Repetition,
};
use tempdir::TempDir;

use super::get_user_name_from_cookie;
use crate::config::{ConfigObject, ConfigTrait};
use crate::{archive_unpacker, common, config, install_task};

#[derive(serde::Serialize)]
struct ModTemplate {
    checksum_md5: String,
    filename: String,
    size_in_megabytes: u64,
}

#[derive(serde::Serialize)]
struct ModManagementTemplateContext {
    flash: Option<String>,
    flash_type: Option<String>,
    mods: Vec<ModTemplate>,
    user_name: String,
}

use rocket::response::{self, Responder, Response};

pub struct AssettoModResponse {
    acmod: config::AssettoMod,
    full_path: PathBuf,
}

impl<'a> Responder<'a> for AssettoModResponse {
    fn respond_to(self, _: &Request) -> response::Result<'a> {
        Response::build()
            .header(ContentType::ZIP)
            .raw_header(
                "Content-Disposition",
                format!("inline; filename=\"{}\"", self.acmod.filename),
            )
            .chunked_body(File::open(&self.full_path).unwrap(), 8192)
            .ok()
    }
}

#[get("/mod_management/download?<hash>")]
pub fn mod_download(
    hash: String,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<AssettoModResponse, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(mod_management)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let config = config_lock.read().unwrap();

    for acmod in config.config.mods.iter() {
        if acmod.checksum_md5 == hash {
            let mod_storage = Path::new(&config.config.mod_storage_location);
            return Ok(AssettoModResponse {
                acmod: acmod.clone(),
                full_path: mod_storage.join(&acmod.filename),
            });
        }
    }

    return Err(Flash::error(
        Redirect::to(uri!(mod_management)),
        "Mod hash not found",
    ));
}

#[get("/mod_management/delete?<hash>")]
pub fn mod_delete(
    hash: String,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(super::login_page_get)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let mut config = config_lock.write().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin {
        return Err(Flash::error(
            Redirect::to(uri!(super::index)),
            "Access denied",
        ));
    }

    if let Err(error) = config.delete_mod(&hash) {
        return Err(Flash::error(Redirect::to(uri!(super::index)), error));
    }

    Ok(Flash::success(
        Redirect::to(uri!(mod_management)),
        "Mod deleted successfully.",
    ))
}

#[get("/mod_management")]
pub fn mod_management(
    flash: Option<FlashMessage<'_, '_>>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Template, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(super::login_page_get)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let config = config_lock.read().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin {
        return Err(Flash::error(
            Redirect::to(uri!(super::index)),
            "Access denied",
        ));
    }

    let mut context = ModManagementTemplateContext {
        flash: None,
        flash_type: None,
        mods: vec![],
        user_name: user_name,
    };

    for acmod in config.config.mods.iter() {
        context.mods.push(ModTemplate {
            checksum_md5: acmod.checksum_md5.clone(),
            filename: acmod.filename.clone(),
            size_in_megabytes: acmod.size_in_bytes / 1024 / 1024,
        });
    }

    if let Some(ref msg) = flash {
        context.flash = Some(msg.msg().to_string());
        if msg.name() == "error" {
            context.flash_type = Some("Error".to_string());
        }
    }
    Ok(Template::render("mod_management", &context))
}

fn install_mod(archive_path: &PathBuf, server_paths: &Vec<String>) {
    let temp_dir = TempDir::new("acsync_server_unpack");
    let temp_dir_output = TempDir::new("acsync_server_install");
    if let Err(error) = temp_dir {
        println!("Error creating temp_dir: {}", error.to_string());
        return;
    }
    if let Err(error) = temp_dir_output {
        println!("Error creating temp_dir: {}", error.to_string());
        return;
    }
    let temp_dir = temp_dir.unwrap();
    let temp_dir_output = temp_dir_output.unwrap();
    let temporary_directory = temp_dir.path();
    let output_directory = temp_dir_output.path();
    let _ = std::fs::create_dir_all(output_directory.join("content/cars"));
    let _ = std::fs::create_dir_all(output_directory.join("content/tracks"));
    let _ = archive_unpacker::unpack_archive(Path::new(archive_path), temporary_directory);
    for task in
        install_task::determine_install_tasks(&common::recursive_ls(temporary_directory)).unwrap()
    {
        let target_path = Path::new(output_directory).join(task.target_path);
        println!("{} -> {:?}", task.source_path, target_path);
        let result = fs_extra::dir::move_dir(task.source_path, target_path, &CopyOptions::new());
        if let Err(error) = result {
            println!("Error while installing mod: {}", error.to_string());
            return;
        }
    }

    let copy_options = fs_extra::file::CopyOptions {
        overwrite: true,
        skip_exist: true,
        buffer_size: 65536,
    };

    // for output_dir in server_paths
    for output_dir_str in server_paths {
        let output_dir = Path::new(output_dir_str);
        for entry in common::recursive_ls(output_directory) {
            let path = Path::new(&entry.path);
            let without_prefix = path.strip_prefix(output_directory).unwrap();
            let target_path = output_dir.join(without_prefix);

            if entry.is_file {
                let mut extension: String = "".to_string();
                if path.extension().is_some() {
                    extension = path.extension().unwrap().to_str().unwrap().to_string();
                }
                let parent_dir_name = path
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                if extension == "acd" || extension == "ini" || parent_dir_name == "data" {
                    let _ = std::fs::create_dir_all(&target_path.parent().unwrap());
                    let _ = fs_extra::file::copy(path, target_path, &copy_options);
                }
            } else {
                let dirname = path.file_name().unwrap().to_str().unwrap();
                if dirname == "skins" {
                    let _ = std::fs::create_dir_all(&target_path);
                }
            }
        }
    }
}

#[post("/mod_management/upload", data = "<data>")]
pub fn mod_upload(
    content_type: &ContentType,
    data: Data,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(super::index)),
            "Access denied",
        ));
    }
    std::mem::drop(cookies);

    let mut config = config_lock.write().unwrap();
    let mod_path = Path::new(&config.config.mod_storage_location);

    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::file("file[]")
            .size_limit(4 * 1024 * 1024 * 1024)
            .repetition(Repetition::fixed(50)),
    ]);

    let multipart_form_data = MultipartFormData::parse(content_type, data, options).unwrap();

    let mut good_mods_count = 0;
    let mut bad_mods_count = 0;
    let archives = multipart_form_data.files.get("file[]");
    let server_paths = config.get_server_paths();
    if let Some(file_fields) = archives {
        for file in file_fields {
            if file.file_name.is_none() {
                bad_mods_count += 1;
                continue;
            }

            let file_name = file.file_name.clone().unwrap();
            let path = std::path::Path::new(&file_name);
            let extension = path.extension();
            if extension.is_none() {
                bad_mods_count += 1;
                continue;
            }

            let extension = extension.unwrap().to_os_string();
            if extension != "7z" && extension != "rar" && extension != "zip" {
                bad_mods_count += 1;
                continue;
            }

            let output_path = mod_path.join(file_name);
            if output_path.exists() {
                bad_mods_count += 1;
                continue;
            }

            let result = std::fs::copy(&file.path, &output_path);
            if let Err(error) = result {
                println!(
                    "Error while copying file from '{:?}' to '{:?}', reason: {}",
                    &file.path, output_path, error
                );
                bad_mods_count += 1;
                continue;
            }

            install_mod(&output_path, &server_paths);
            good_mods_count += 1;
        }
    }

    if let Err(error) = config.rebuild_mod_storage(false) {
        return Err(Flash::error(
            Redirect::to(uri!(mod_management)),
            format!(
                "{} mods uploaded successfully, {} failed. Error while rebuilding mod storage: {}.",
                good_mods_count, bad_mods_count, error
            ),
        ));
    }

    if bad_mods_count > 0 {
        return Err(Flash::error(
            Redirect::to(uri!(mod_management)),
            format!(
                "{} mods uploaded successfully, {} failed",
                good_mods_count, bad_mods_count
            ),
        ));
    }

    Ok(Flash::success(
        Redirect::to(uri!(mod_management)),
        format!("{} mods uploaded successfully.", good_mods_count),
    ))
}
