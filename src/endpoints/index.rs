use std::collections::HashMap;
use std::sync::RwLock;

use rocket::http::Cookies;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_contrib::templates::Template;

use crate::config::{ConfigObject, ConfigTrait};

#[get("/")]
pub fn index(
    flash: Option<FlashMessage<'_, '_>>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Template, Flash<Redirect>> {
    let user_name = super::get_user_name_from_cookie(&mut cookies);
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
