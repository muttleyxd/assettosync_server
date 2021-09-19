use std::collections::HashMap;
use std::sync::RwLock;

use rocket::http::Cookies;
use rocket::request::{FlashMessage, Form};
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_contrib::templates::Template;

use crate::config::{ConfigObject, ConfigTrait};

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

#[get("/user_management")]
pub fn user_management(
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

    if !is_admin {
        return Err(Flash::error(
            Redirect::to(uri!(super::index)),
            "Access denied",
        ));
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

#[get("/user_management/change_password?<username>")]
pub fn user_management_change_password_get(
    flash: Option<FlashMessage<'_, '_>>,
    mut cookies: Cookies,
    username: String,
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

    if !is_admin && username != user_name {
        return Err(Flash::error(
            Redirect::to(uri!(super::index)),
            "Access denied",
        ));
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
pub struct ChangePasswordData {
    login: String,
    password1: String,
    password2: String,
}

#[post("/user_management/change_password", data = "<data>")]
pub fn user_management_change_password_post(
    data: Form<ChangePasswordData>,
    mut cookies: Cookies,
    config_lock: State<RwLock<ConfigObject>>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let user_name = super::get_user_name_from_cookie(&mut cookies);
    if user_name.is_none() {
        return Err(Flash::error(
            Redirect::to(uri!(super::index)),
            "Access denied",
        ));
    }
    std::mem::drop(cookies);

    let mut config = config_lock.write().unwrap();
    let user_name = user_name.unwrap();
    let is_admin = config.is_user_admin(&user_name);

    if !is_admin && data.login != user_name {
        return Err(Flash::error(
            Redirect::to(uri!(super::index)),
            "Access denied",
        ));
    }

    if data.password1 != data.password2 {
        return Err(Flash::error(
            Redirect::to(uri!(user_management_change_password_get: data.login.clone())),
            "Passwords are not the same",
        ));
    }

    let result = config.change_user_password(&data.login, &data.password1);
    if let Err(error) = result {
        return Err(Flash::error(
            Redirect::to(uri!(user_management_change_password_get: data.login.clone())),
            error,
        ));
    }

    if !is_admin {
        return Ok(Flash::success(
            Redirect::to(uri!(super::index)),
            "Password changed",
        ));
    }
    Ok(Flash::success(
        Redirect::to(uri!(user_management)),
        "Password changed",
    ))
}
