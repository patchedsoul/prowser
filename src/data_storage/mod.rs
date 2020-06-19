mod download;

use crate::markdown;

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::path::Path;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns content of a local file.
pub fn open_local_file(path: &str) -> Result<String, String> {
    let mut content = String::new();
    File::open(&path)
        .map_err(|e| e.to_string())?
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())?;
    Ok(content)
}

/// Downloads file (if not cached).
/// Returns relative file system path.
pub fn download_cache_path(url: &str, accepted_mime_types: Vec<&str>) -> Result<String, String> {
    let mut s = DefaultHasher::new();
    url.hash(&mut s);

    let path = format!("cache/{}", s.finish());

    let mut mime_type = String::new();

    if let Some(mime) = file_cached(&path) {
        mime_type = mime;
    } else {
        let mut responce = download::request(url)?;

        let mut out = fs::File::create(&path).map_err(|e| e.to_string())?;
        io::copy(&mut responce, &mut out).map_err(|e| e.to_string())?;

        let headers = responce.headers();
        let content_type = headers
            .get("content-type")
            .and_then(|value| value.to_str().ok());

        if let Some(responce_mime_type) = content_type {
            let cache_control = headers
                .get("cache-control")
                .and_then(|value| value.to_str().ok());

            if let Some(responce_cache_control) = cache_control {
                mime_type.push_str(responce_mime_type);

                add_to_cache(responce_mime_type, &path, responce_cache_control);
            }
        }
    }

    // check mime type
    if check_mimetype(&mime_type, accepted_mime_types) {
        Ok(path)
    } else {
        Err(path)
    }
}

/// Downloads file (if not cached) and returns content.
/// On wrong mime type, return error with path to file.
pub fn download_and_get(url: &str, accepted_mime_types: Vec<&str>) -> Result<String, String> {
    let mut s = DefaultHasher::new();
    url.hash(&mut s);

    let path = format!("cache/{}", s.finish());

    let mut mime_type = String::new();

    if let Some(mime) = file_cached(&path) {
        mime_type = mime;
    } else {
        let mut responce = download::request(url)?;

        let mut out = fs::File::create(&path).map_err(|e| e.to_string())?;
        io::copy(&mut responce, &mut out).map_err(|e| e.to_string())?;

        let headers = responce.headers();
        let content_type = headers
            .get("content-type")
            .and_then(|value| value.to_str().ok());

        if let Some(responce_mime_type) = content_type {
            let cache_control = headers
                .get("cache-control")
                .and_then(|value| value.to_str().ok());

            if let Some(responce_cache_control) = cache_control {
                mime_type.push_str(responce_mime_type);

                add_to_cache(responce_mime_type, &path, responce_cache_control);
            }
        }
    }

    // check mime type
    if check_mimetype(&mime_type, accepted_mime_types) {
        Ok(open_local_file(&path).expect("File to be freshly downloaded or already cached"))
    } else {
        Err(path)
    }
}

/// Downloads file (if not cached) and returns content.
/// On wrong mime type, return error with path to file.
pub fn download(url: &str) -> Result<(reqwest::blocking::Response, String), String> {
    let mut s = DefaultHasher::new();
    url.hash(&mut s);

    let path = format!("cache/{}", s.finish());

    // download -> responce
    let mut responce = download::request(url)?;

    let mut out = fs::File::create(&path).map_err(|e| e.to_string())?;
    io::copy(&mut responce, &mut out).map_err(|e| e.to_string())?;

    // after saving responce to file, text() is empty
    Ok((responce, path))
}

/// Downloads file (if not cached) with given parameters and returns content.
pub fn download_and_get_post(url: &str, params: &[(&str, &str)]) -> String {
    let mut s = DefaultHasher::new();
    url.hash(&mut s);

    let path = format!("cache/{}", s.finish());
    match download::save_file_post(url, &path, params) {
        Ok(()) => open_local_file(&path).unwrap(),
        Err(error) => open_error_document(error),
    }
}

/// downloads
/// return html
/// either directly, text, converted md or image
pub fn for_tab(url: &str) -> String {
    let download = download(url);

    match download {
        Ok((responce, path)) => {
            let headers = responce.headers();
            let content_type = headers
                .get("content-type")
                .and_then(|value| value.to_str().ok());

            if let Some(mime_type) = content_type {
                if mime_type.starts_with("text/html") {
                    open_local_file(&path).unwrap()
                } else if mime_type.starts_with("text/plain")
                    || mime_type.starts_with("text/css")
                    || mime_type.starts_with("text/javascript")
                    || mime_type.starts_with("application/javascript")
                {
                    let mut content = open_local_file(&path).unwrap();
                    let template =
                        open_local_file("assets/text.html").expect("'text' asset to be present");

                    content = content.replace("\n", "<br>");

                    // FIXME: escape content for possible html elements
                    template.replacen("replace_body", &content, 1)
                } else if mime_type.starts_with("text/markdown") {
                    let content = open_local_file(&path).unwrap();
                    let template = open_local_file("assets/markdown.html")
                        .expect("'markdown' asset to be present");

                    // FIXME: probably should give real url
                    let markdown = markdown::parse(content, String::new());

                    // FIXME: use selected stylesheets from config
                    template.replacen("replace_body", &markdown, 1)
                } else if mime_type.starts_with("image/jpeg")
                    || mime_type.starts_with("image/gif")
                    || mime_type.starts_with("image/png")
                    || mime_type.starts_with("image/webp")
                {
                    let template =
                        open_local_file("assets/image.html").expect("'image' asset to be present");

                    template.replacen("replace_image", url, 3)
                } else {
                    save_to_downloads(&path);
                    format!("Unsuported Mime Type: {}. Saved to downloads", mime_type)
                }
            } else {
                save_to_downloads(&path);
                String::from("No Mime Type specified. Saved to downloads")
            }
        }
        // download falsch
        Err(error) => open_error_document(error),
    }
}

fn save_to_downloads(cache_path: &str) {
    let vec = cache_path.split('/').collect::<Vec<&str>>()[1];
    // FIXME: fix for windows and change to real filename
    let username = std::process::Command::new("whoami")
        .output()
        .expect("wohami command failed to start")
        .stdout;
    let mut username = str::from_utf8(&username).unwrap().to_string();
    username.truncate(username.len() - 1); // remove '\n'

    let mut destination = format!("/home/{}/Downloads/", username);
    destination.push_str(vec);
    fs::copy(cache_path, &destination).expect("Error when copying in downloads directory");
}

// FIXME: files that are not valid or do not exist anymore should be deleted and removed from cache file
/// checks if a file is cached and still valid
fn file_cached(name: &str) -> Option<String> {
    let mut content = String::new();
    File::open("cache/cache.csv")
        .map_err(|e| e.to_string())
        .expect("'cache' asset to be present")
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())
        .unwrap();

    let lines = content.split('\n').collect::<Vec<&str>>();

    for line in lines {
        // path/to/cache|mime_type|cache_control|download_time
        let cells = line.split('|').collect::<Vec<&str>>();

        if cells[0] == name {
            // file was cached some time ago

            if let Ok(n) = SystemTime::now().duration_since(UNIX_EPOCH) {
                // FIXME: read cache_control max-age. ↓ Don't assume a year
                if cells[3].parse::<u64>().unwrap() + 31_536_000 > n.as_secs() {
                    // file is still valid

                    if Path::new(cells[0]).exists() {
                        // file on disk still exists
                        return Some(cells[1].to_string());
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
        }
    }

    None
}

fn add_to_cache(hash: &str, mime_type: &str, cache_control: &str) {
    // add file entry to cache
    // with hashed path
    // mime type
    // ...

    let mut file = OpenOptions::new()
        .append(true)
        .open("cache/cache.csv")
        .expect("'cache' asset to be present");

    if let Ok(n) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let line = format!("{}|{}|{}|{}\n", mime_type, hash, cache_control, n.as_secs());
        // FIXME: never add "must revalidate" or similar
        let _ = file.write_all(line.as_bytes());
    }
}

/// Checks if mimetypes contains accepted mime type.
fn check_mimetype(mime_type: &str, accepted_mime_types: Vec<&str>) -> bool {
    for accepted_type in accepted_mime_types {
        if mime_type.contains(accepted_type) {
            return true;
        }
    }

    false
}

/// Return error document on given http error
fn open_error_document(error: String) -> String {
    if error.contains("lookup address information: Name or service not known") {
        open_local_file("assets/server-not-found.html")
            .expect("'server not found' asset to be present")
    } else if error == "404 Not Found" {
        open_local_file("assets/404-not-found.html").expect("'404' asset to be present")
    } else {
        error
    }
}

#[cfg(test)]
mod open {
    use super::*;

    #[test]
    fn error_document() {
        assert_eq!(
            open_error_document(String::from("404 Not Found")),
            open_local_file("assets/404-not-found.html").unwrap()
        );
    }

    #[test]
    fn error_document_no_match() {
        assert_eq!(
            open_error_document(String::from("unknown error")),
            String::from("unknown error")
        );
    }
}
