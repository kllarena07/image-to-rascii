use actix_web::{App, HttpRequest, HttpResponse, HttpServer, get, web, http::header};
use rascii_art::{RenderOptions, render_to};
use regex::Regex;
use reqwest::Client;
use std::env;
use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};

async fn convert_image_to_rascii(filepath: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut buffer = String::new();

    render_to(
        filepath.to_str().ok_or("Invalid filepath encoding")?,
        &mut buffer,
        &RenderOptions::new()
            .height(30)
            .colored(true)
            .charset(rascii_art::charsets::BLOCK),
    )
    .map_err(|e| format!("Failed to render image {}: {}", filepath.display(), e))?;

    Ok(buffer)
}

async fn download_image(url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to send HTTP request: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download image: HTTP {}", response.status()).into());
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    eprintln!("Downloaded image data: {} bytes", bytes.len());

    // Extract filename from URL
    let raw_filename = url
        .rsplit('/')
        .next()
        .and_then(|s| s.split('?').next())
        .unwrap_or("");
    let filename = Path::new(raw_filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Could not extract a valid filename from URL")?;
    if filename.is_empty() {
        return Err("Filename extracted from URL is empty".into());
    }

    // Create images directory if it doesn't exist
    let mut filepath =
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    filepath.push("images");
    fs::create_dir_all(&filepath)
        .map_err(|e| format!("Failed to create images directory: {}", e))?;
    filepath.push(filename);

    eprintln!("Attempting to save image to: {}", filepath.display());
    let mut file = File::create(&filepath)
        .map_err(|e| format!("Failed to create file {}: {}", filepath.display(), e))?;

    copy(&mut bytes.as_ref(), &mut file).map_err(|e| {
        format!(
            "Failed to write image data to {}: {}",
            filepath.display(),
            e
        )
    })?;

    eprintln!("Successfully saved image to: {}", filepath.display());
    Ok(filepath)
}

#[get("/{url:.*}")]
async fn index(req: HttpRequest, path: web::Path<String>) -> Result<HttpResponse, actix_web::Error> {
    let url = path.into_inner();

    // Check User-Agent header
    let user_agent = req.headers().get(header::USER_AGENT).and_then(|h| h.to_str().ok()).unwrap_or("");
    let is_terminal = user_agent.to_lowercase().contains("curl") || user_agent.to_lowercase().contains("wget");

    if !is_terminal {
        return Ok(HttpResponse::Ok()
            .content_type("text/plain")
            .body("RASCII art is only available for terminal clients. Please use curl or a similar tool."));
    }

    let img_re = Regex::new(r"(?i)\.(jpg|jpeg|png|gif|bmp|webp|tiff)(\?.*)?$").unwrap();
    if !img_re.is_match(&url) {
        return Ok(HttpResponse::BadRequest().body("Provided URL is not an image file."));
    }
    let filepath = download_image(&url).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to download image: {}", e))
    })?;

    eprintln!(
        "Attempting to convert image from path: {}",
        filepath.display()
    );
    let rascii = convert_image_to_rascii(&filepath).await?;
    Ok(HttpResponse::Ok().content_type("text/plain; charset=utf-8").body(rascii))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Ensure images directory exists
    let mut images_dir = env::current_dir()?;
    images_dir.push("images");
    fs::create_dir_all(&images_dir)?;

    println!("Starting server on 127.0.0.1:8080");
    let server = HttpServer::new(|| App::new().service(index)).bind(("127.0.0.1", 8080));

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
