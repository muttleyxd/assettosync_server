use rocket::response::content;

#[get("/style.css")]
pub fn style_css() -> content::Css<&'static str> {
    content::Css(include_str!("../resources/style.css"))
}
