use std::fs;
use std::io;
use std::path::Path;

fn get_headers() -> reqwest::header::HeaderMap {
    /*
    https://httpbin.org/get

    Time Zone : 0 UTC+1
    Referer
    */
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; rv:68.0) Gecko/20100101 Firefox/68.0",
        ),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static(
            "text/html, application/xhtml+xml, application/xml;q=0.9, */*;q=0.8",
        ),
    );
    headers.insert(
        reqwest::header::DNT,
        reqwest::header::HeaderValue::from_static("1"),
    );
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        reqwest::header::HeaderValue::from_static("en-us,en;q=0.5"),
    );
    headers.insert(
        reqwest::header::UPGRADE_INSECURE_REQUESTS,
        reqwest::header::HeaderValue::from_static("1"),
    );

    headers
}

// https://www.reddit.com/r/rust/comments/9lrpru/download_file_with_progress_bar/
pub fn request(url: &str) -> Result<reqwest::blocking::Response, String> {
    // TODO: only use 1 single client and reuse it.

    let client = reqwest::blocking::Client::builder()
        //.cookie_store(true) <- currently useless as I create a new `Client` for each request
        .referer(false)
        .default_headers(get_headers())
        .build()
        .map_err(|e| e.to_string())?;
    let responce = client.get(url).send().map_err(|e| e.to_string())?;

    // FIXME: may require a single client. Seperate cookie store is needed anyway
    //dbg!(responce.cookies().collect::<Vec<_>>());

    Ok(responce)
}

/// downloads a file with given parameters
// https://www.reddit.com/r/rust/comments/9lrpru/download_file_with_progress_bar/
pub fn save_file_post(url: &str, path: &str, params: &[(&str, &str)]) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        //.cookie_store(true) <- currently useless as I create a new `Client` for each request
        .referer(false)
        .default_headers(get_headers())
        .build()
        .map_err(|e| e.to_string())?;

    let mut responce = client
        .post(url)
        .form(&params)
        .send()
        .map_err(|e| e.to_string())?;

    let status = responce.status();
    if !status.is_success() {
        let text = responce.text().unwrap();
        return Err(if text.is_empty() {
            status.to_string()
        } else {
            text
        });
    }

    if !Path::new("cache/").exists() {
        fs::create_dir("cache").map_err(|e| e.to_string())?;
    }
    let mut out = fs::File::create(path).map_err(|e| e.to_string())?;
    io::copy(&mut responce, &mut out).map_err(|e| e.to_string())?;

    Ok(())
}
