use actix_web::HttpRequest;
use actix_files::NamedFile;

#[get("/palette")]
fn handler(_req: HttpRequest) -> std::io::Result<NamedFile> {
    actix_files::NamedFile::open("pages/palette.html")
}
