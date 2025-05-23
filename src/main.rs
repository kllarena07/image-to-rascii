use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use regex::Regex;

#[get("/{url:.*}")]
async fn convert_to_rascii(path: web::Path<String>) -> impl Responder {
    let url = path.into_inner();
    // Regex for common image extensions (case-insensitive, allows for querystrings)
    let img_re = Regex::new(r"(?i)\.(jpg|jpeg|png|gif|bmp|webp|tiff)(\?.*)?$").unwrap();
    if !img_re.is_match(&url) {
        return HttpResponse::BadRequest().body("Provided URL is not an image file.");
    }
    HttpResponse::Ok().body(url)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server on 127.0.0.1:8080");
    let server = HttpServer::new(|| App::new().service(convert_to_rascii))
        .bind(("127.0.0.1", 8080));

    let server = match server {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind server: {}", e);
            return Err(e);
        }
    };

    if let Err(e) = server.run().await {
        eprintln!("Server error: {}", e);
        return Err(e);
    }
    Ok(())
}
