use actix_web::{App, HttpRequest, HttpResponse, HttpServer, get, http::header, web};
use image::load_from_memory;
use rascii_art::{RenderOptions, render_image_to}; // Changed from render_bytes_to
use regex::Regex;
use reqwest::Client;

async fn convert_image_to_rascii(
    image_bytes: &[u8],
    use_color: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut buffer = String::new();

    let img = load_from_memory(image_bytes)
        .map_err(|e| format!("Failed to load image from memory: {}", e))?;

    render_image_to(
        &img,
        &mut buffer,
        &RenderOptions::new()
            .height(30)
            .colored(use_color)
            .charset(rascii_art::charsets::BLOCK),
    )
    .map_err(|e| format!("Failed to render image from buffer: {}", e))?;

    Ok(buffer)
}

async fn download_image(url: &str) -> Result<web::Bytes, Box<dyn std::error::Error>> {
    // Changed PathBuf to Bytes
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

    // Removed file system operations

    Ok(bytes) // Return the Bytes directly
}

#[get("/{url:.*}")]
async fn index(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let url = path.into_inner();

    // Image validation first
    let img_re = Regex::new(r"(?i)\.(jpg|jpeg|png|gif|bmp|webp|tiff)(\?.*)?$").unwrap();
    if !img_re.is_match(&url) {
        eprintln!("Validation failed: URL is not an image file: {}", url);
        return Ok(HttpResponse::BadRequest().body("Provided URL is not an image file."));
    }

    // Download image
    let image_bytes = download_image(&url).await.map_err(|e| {
        eprintln!("Failed to download image from URL {}: {}", url, e);
        actix_web::error::ErrorInternalServerError(format!("Failed to download image: {}", e))
    })?;

    // Determine if color should be used based on User-Agent
    let mut use_color = true; // Default to true (for curl, etc.)
    if let Some(user_agent_header) = req.headers().get(header::USER_AGENT) {
        if let Ok(user_agent_str) = user_agent_header.to_str() {
            let ua_lower = user_agent_str.to_lowercase();
            if ua_lower.contains("mozilla")
                || ua_lower.contains("chrome")
                || ua_lower.contains("safari")
                || ua_lower.contains("firefox")
                || ua_lower.contains("edge")
            {
                use_color = false; // It's a browser
            }
        }
    }

    // Convert to RASCII or ASCII
    eprintln!(
        "Attempting to convert image from buffer ({} bytes) (color: {})",
        image_bytes.len(),
        use_color
    );
    let art_result = convert_image_to_rascii(&image_bytes, use_color) // Pass image_bytes slice
        .await
        .map_err(|e| {
            eprintln!(
                "Failed to convert image from buffer to art ({} bytes): {}",
                image_bytes.len(),
                e
            );
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to convert image to art: {}",
                e
            ))
        })?;

    if use_color {
        // Non-browser: Respond with plain text RASCII
        eprintln!("Responding with plain text RASCII for URL: {}.", url);
        Ok(HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body(art_result))
    } else {
        // Browser: Respond with HTML-formatted ASCII
        eprintln!("Responding with HTML-formatted ASCII for URL: {}.", url);
        let html_body = format!("<pre>{}</pre>", art_result);
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html_body))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Removed images directory creation as it's no longer needed

    println!("Starting server on 0.0.0.0:8080");
    let server = HttpServer::new(|| App::new().service(index)).bind(("0.0.0.0", 8080));

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
