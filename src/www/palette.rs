use actix_web::HttpRequest;

#[get("/palette")]
fn handler(_req: HttpRequest) -> &'static str {
    "Coming soon"
}
