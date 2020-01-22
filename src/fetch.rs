extern crate http;

use http::StatusCode;
use std::fmt;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use url::{ParseError, Url};

use super::parse;
use super::parse::Link;

const TIMEOUT: u64 = 10;

#[derive(Debug, Clone)]
pub enum UrlState {
    Accessible(Url),
    BadStatus(Url, StatusCode),
    ConnectionFailed(Url),
    TimedOut(Url),
    Malformed(String),
}

impl fmt::Display for UrlState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            UrlState::Accessible(ref url) => format!("!! {}", url).fmt(f),
            UrlState::BadStatus(ref url, ref status) => format!("x {} ({})", url, status).fmt(f),
            UrlState::ConnectionFailed(ref url) => format!("x {} (connection failed)", url).fmt(f),
            UrlState::TimedOut(ref url) => format!("x {} (timed out)", url).fmt(f),
            UrlState::Malformed(ref url) => format!("x {} (malformed)", url).fmt(f),
        }
    }
}

pub fn build_url(domain: &str, path: &str) -> Result<Url, ParseError> {
    let base_url_string = format!("https://{}", domain);
    let base_url = Url::parse(&base_url_string).unwrap();
    let options = Url::options().base_url(Some(&base_url));
    options.parse(path)
}

pub fn url_status(domain: &str, path: &str) -> UrlState {
    match build_url(domain, path) {
        Ok(url) => {
            let (tx, rx) = channel();
            let req_tx = tx.clone();
            let u = url.clone();

            thread::spawn(move || {
                let url_string = url.as_str();
                let resp = reqwest::blocking::get(url_string);

                let _ = req_tx.send(match resp {
                    Ok(r) => {
                        if let StatusCode::OK = r.status() {
                            UrlState::Accessible(url)
                        } else {
                            UrlState::BadStatus(url, r.status())
                        }
                    }
                    Err(_) => UrlState::ConnectionFailed(url),
                });
            });

            thread::spawn(move || {
                thread::sleep(Duration::from_secs(TIMEOUT));
                let _ = tx.send(UrlState::TimedOut(u));
            });

            rx.recv().unwrap()
        }
        Err(_) => UrlState::Malformed(path.to_owned()),
    }
}

pub fn fetch_url(url: &Url) -> Result<String, Box<dyn std::error::Error>> {
    let url_string = url.as_str();
    let res = reqwest::blocking::get(url_string)?;
    let body = res.text()?;
    Ok(body)
}

pub fn fetch_all_links(
    url: &Url,
    domain: &str,
) -> Result<Vec<Arc<Link>>, Box<dyn std::error::Error>> {
    let html_src = fetch_url(url)?.to_string();
    let dom = parse::parse_html(&html_src);

    Ok(parse::get_links(dom.document, domain, url.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_build_urls() -> Result<(), ParseError> {
        let url_string = build_url("google.com", "/home")?.to_string();
        assert_eq!(url_string, "https://google.com/home");
        Ok(())
    }
}
