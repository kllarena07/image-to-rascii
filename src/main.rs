use actix_web::{App, HttpResponse, HttpServer, get, web};
use regex::Regex;
use reqwest::Client;
use std::fs::File;
use std::io::copy;
use std::path::Path;

async fn download_image(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to download image: HTTP {}", response.status()).into());
    }

    let bytes = response.bytes().await?;

    // Extract filename from URL
    let filename = url
        .rsplit('/')
        .next()
        .and_then(|s| {
            let s = s.split('?').next().unwrap_or("");
            if s.is_empty() { None } else { Some(s) }
        })
        .ok_or("Could not extract a valid filename from URL")?;

    let filepath = format!("./{}", filename);
    let mut file = File::create(Path::new(&filepath))?;
    copy(&mut bytes.as_ref(), &mut file)?;

    Ok(())
}

#[get("/{url:.*}")]
async fn convert_to_rascii(path: web::Path<String>) -> Result<HttpResponse, actix_web::Error> {
    let url = path.into_inner();
    let img_re = Regex::new(r"(?i)\.(jpg|jpeg|png|gif|bmp|webp|tiff)(\?.*)?$").unwrap();
    if !img_re.is_match(&url) {
        return Ok(HttpResponse::BadRequest().body("Provided URL is not an image file."));
    }
    download_image(&url).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to download image: {}", e))
    })?;
    Ok(HttpResponse::Ok().body("Image downloaded successfully"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server on 127.0.0.1:8080");
    let server =
        HttpServer::new(|| App::new().service(convert_to_rascii)).bind(("127.0.0.1", 8080));

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
