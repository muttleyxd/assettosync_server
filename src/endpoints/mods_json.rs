use std::sync::RwLock;

use rocket::http::Cookies;
use rocket::State;
use rocket_contrib::json::Json;

use crate::config::ConfigObject;

#[derive(serde::Serialize)]
pub struct JsonModTemplate {
    checksum_md5: String,
    filename: String,
    size_in_bytes: u64,
}

#[get("/mods.json")]
pub fn mods_json(
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Json<Vec<JsonModTemplate>>, &'static str> {
    let user_name = super::get_user_name_from_cookie(&mut cookies);
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
