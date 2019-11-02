use actix_web::{ HttpRequest, HttpResponse };

#[get("/")]
fn handler(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body(
        "<a href=\"library\">Library</a><br>
        <a href=\"palette\">Palette</a>"
    )
}
