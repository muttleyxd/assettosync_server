use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use rocket::http::{ContentType, Cookie, Cookies};
use rocket::request::{FlashMessage, Form, Request};
use rocket::response::{content, Flash, Redirect};
use rocket::{Data, Route, State};
use rocket_contrib::{json::Json, templates::Template};
use rocket_multipart_form_data::{
    MultipartFormData, MultipartFormDataField, MultipartFormDataOptions, Repetition,
};
use tempdir::TempDir;

use crate::{archive_unpacker, common, config, install_task};
use crate::config::{ConfigObject, ConfigTrait};

fn get_user_name_from_cookie(cookies: &mut Cookies) -> Option<String> {
    let cookie = cookies.get_private("user_name");
    match cookie {
        Some(cookie) => Some(cookie.value().to_string()),
        None => None,
    }
}

#[get("/")]
fn index(
    flash: Option<FlashMessage<'_, '_>>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Template, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(login_page_get)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let config = config_lock.read().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    let mut context = HashMap::new();
    context.insert("user_name", user_name);
    if is_admin {
        context.insert("admin", is_admin.to_string());
    }
    if let Some(ref msg) = flash {
        context.insert("flash", msg.msg().to_string());
        if msg.name() == "error" {
            context.insert("flash_type", "Error".to_string());
        }
    }
    Ok(Template::render("index", &context))
}

#[get("/login")]
fn login_page_get(flash: Option<FlashMessage<'_, '_>>) -> Template {
    let mut context = HashMap::new();
    if let Some(ref msg) = flash {
        context.insert("flash", msg.msg());
        if msg.name() == "error" {
            context.insert("flash_type", "Error");
        }
    }

    Template::render("login", &context)
}

#[derive(FromForm)]
struct LoginData {
    login: String,
    password: String,
}

#[post("/login", data = "<data>")]
fn login_page_post(
    data: Form<LoginData>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Redirect, Flash<Redirect>> {
    let config = config_lock.read().unwrap();

    let authentication_success = config.is_login_data_valid(&data.login, &data.password);
    if !authentication_success {
        return Err(Flash::error(
            Redirect::to(uri!(login_page_get)),
            "Wrong username/password",
        ));
    }
    cookies.add_private(Cookie::new("user_name", data.login.clone()));
    Ok(Redirect::to(uri!(index)))
}

#[post("/logout")]
fn logout(mut cookies: Cookies) -> Redirect {
    cookies.remove_private(Cookie::named("user_name"));
    Redirect::to(uri!(login_page_get))
}

#[get("/style.css")]
fn style_css() -> content::Css<&'static str> {
    content::Css(include_str!("../resources/style.css"))
}

#[derive(serde::Serialize)]
struct UserTemplate {
    login: String,
    is_admin: bool,
}

#[derive(serde::Serialize)]
struct UserManagementTemplateContext {
    flash: Option<String>,
    flash_type: Option<String>,
    user_name: String,
    users: Vec<UserTemplate>,
}

#[get("/change_password?<username>")]
fn change_password_get(
    flash: Option<FlashMessage<'_, '_>>,
    mut cookies: Cookies,
    username: String,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Template, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(login_page_get)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let config = config_lock.read().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin && username != user_name {
        return Err(Flash::error(Redirect::to(uri!(index)), "Access denied"));
    }

    let mut context = HashMap::new();
    context.insert("user_name", username);
    if is_admin {
        context.insert("is_admin", "yes".to_string());
    }
    if let Some(ref msg) = flash {
        context.insert("flash", msg.msg().to_string());
        if msg.name() == "error" {
            context.insert("flash_type", "Error".to_string());
        }
    }
    Ok(Template::render("change_password", &context))
}

#[derive(FromForm)]
struct ChangePasswordData {
    login: String,
    password1: String,
    password2: String,
}

#[post("/change_password", data = "<data>")]
fn change_password_post(
    data: Form<ChangePasswordData>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(Redirect::to(uri!(index)), "Access denied"));
    }
    std::mem::drop(cookies);

    let mut config = config_lock.write().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin && data.login != user_name {
        return Err(Flash::error(Redirect::to(uri!(index)), "Access denied"));
    }

    if data.password1 != data.password2 {
        return Err(Flash::error(
            Redirect::to(uri!(change_password_get: data.login.clone())),
            "Passwords are not the same",
        ));
    }

    let result = config.change_user_password(&data.login, &data.password1);
    if let Err(error) = result {
        return Err(Flash::error(
            Redirect::to(uri!(change_password_get: data.login.clone())),
            error,
        ));
    }

    if !is_admin {
        return Ok(Flash::success(
            Redirect::to(uri!(index)),
            "Password changed",
        ));
    }
    Ok(Flash::success(
        Redirect::to(uri!(user_management)),
        "Password changed",
    ))
}

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

struct AssettoModResponse {
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
fn mod_download(
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
fn mod_delete(
    hash: String,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(login_page_get)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let mut config = config_lock.write().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin {
        return Err(Flash::error(Redirect::to(uri!(index)), "Access denied"));
    }

    if let Err(error) = config.delete_mod(&hash) {
        return Err(Flash::error(Redirect::to(uri!(index)), error));
    }

    Ok(Flash::success(
        Redirect::to(uri!(mod_management)),
        "Mod deleted successfully.",
    ))
}

#[get("/mod_management")]
fn mod_management(
    flash: Option<FlashMessage<'_, '_>>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Template, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(login_page_get)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let config = config_lock.read().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin {
        return Err(Flash::error(Redirect::to(uri!(index)), "Access denied"));
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

#[derive(serde::Serialize)]
struct JsonModTemplate {
    checksum_md5: String,
    filename: String,
    size_in_bytes: u64,
}

#[get("/mods.json")]
fn mods_json(
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Json<Vec<JsonModTemplate>>, &'static str> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err("Not logged in");
    }
    std::mem::drop(cookies);

    let config = config_lock.read().unwrap();

    let mut mods = vec![];
    for acmod in config.config.mods.iter() {
        mods.push(JsonModTemplate {
            checksum_md5: acmod.checksum_md5.clone(),
            filename: acmod.filename.clone(),
            size_in_bytes: acmod.size_in_bytes,
        });
    }
    Ok(Json(mods))
}

use fs_extra::dir::CopyOptions;

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
fn mod_upload(
    content_type: &ContentType,
    data: Data,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(Redirect::to(uri!(index)), "Access denied"));
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

#[get("/user_management")]
fn user_management(
    flash: Option<FlashMessage<'_, '_>>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Template, Flash<Redirect>> {
    let user_name = get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(login_page_get)),
            "You need to be logged in to view this page",
        ));
    }
    std::mem::drop(cookies);

    let config = config_lock.read().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin {
        return Err(Flash::error(Redirect::to(uri!(index)), "Access denied"));
    }

    let mut context = UserManagementTemplateContext {
        flash: None,
        flash_type: None,
        user_name: user_name,
        users: vec![],
    };

    for user in config.config.users.iter() {
        context.users.push(UserTemplate {
            login: user.login.clone(),
            is_admin: user.is_admin,
        });
    }

    if let Some(ref msg) = flash {
        context.flash = Some(msg.msg().to_string());
        if msg.name() == "error" {
            context.flash_type = Some("Error".to_string());
        }
    }
    Ok(Template::render("user_management", &context))
}

pub fn get_routes() -> Vec<Route> {
    routes![
        change_password_get,
        change_password_post,
        index,
        login_page_get,
        login_page_post,
        logout,
        mod_delete,
        mod_download,
        mod_management,
        mod_upload,
        mods_json,
        style_css,
        user_management
    ]
}
