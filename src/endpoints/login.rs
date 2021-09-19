use std::collections::HashMap;
use std::sync::RwLock;

use crate::config::{ConfigObject, ConfigTrait};
use rocket::http::{Cookie, Cookies};
use rocket::request::{FlashMessage, Form};
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_contrib::templates::Template;

#[get("/login")]
pub fn login_page_get(flash: Option<FlashMessage<'_, '_>>) -> Template {
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
pub struct LoginData {
    login: String,
    password: String,
}

#[post("/login", data = "<data>")]
pub fn login_page_post(
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
    Ok(Redirect::to(uri!(super::index::index)))
}

#[post("/logout")]
pub fn logout(mut cookies: Cookies) -> Redirect {
    cookies.remove_private(Cookie::named("user_name"));
    Redirect::to(uri!(login_page_get))
}
