use rocket::http::Cookies;
use rocket::Route;

mod index;
mod login;
mod mod_management;
mod mods_json;
mod style_css;
mod user_management;

use index::*;
use login::*;
use mod_management::*;
use mods_json::*;
use style_css::*;
use user_management::*;

fn get_user_name_from_cookie(cookies: &mut Cookies) -> Option<String> {
    let cookie = cookies.get_private("user_name");
    match cookie {
        Some(cookie) => Some(cookie.value().to_string()),
        None => None,
    }
}

pub fn get_routes() -> Vec<Route> {
    routes![
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
        user_management,
        user_management_change_password_get,
        user_management_change_password_post,
    ]
}
