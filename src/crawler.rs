use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use url::Url;

use super::fetch::{fetch_all_links, url_status, UrlState};
use super::parse::Link;

const THREADS: i32 = 20;

pub struct Crawler {
    to_visit: Arc<Mutex<Vec<String>>>,
    active_count: Arc<Mutex<i32>>,
    url_states: Receiver<UrlState>,
    link_data: Arc<Mutex<HashMap<String, Arc<Link>>>>,
}

impl Iterator for Crawler {
    type Item = (UrlState, Option<Arc<Link>>);

    fn next(&mut self) -> Option<(UrlState, Option<Arc<Link>>)> {
        loop {
            match self.url_states.try_recv() {
                Ok(state) => match state {
                    UrlState::BadStatus(ref url, _) => {
                        let link_data_val = self.link_data.lock().unwrap();
                        let maybe_link = match link_data_val.get(url.as_str()) {
                            Some(link) => Some(link.clone()),
                            None => None,
                        };
                        return Some((state, maybe_link));
                    }
                    _ => return Some((state, None)),
                },
                Err(_) => {
                    let to_visit_val = self.to_visit.lock().unwrap();
                    let active_count_val = self.active_count.lock().unwrap();

                    if to_visit_val.is_empty() && *active_count_val == 0 {
                        return None;
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}

fn crawl_worker_thread(
    domain: &str,
    to_visit: Arc<Mutex<Vec<String>>>,
    visited: Arc<Mutex<HashSet<String>>>,
    active_count: Arc<Mutex<i32>>,
    url_states: Sender<UrlState>,
    link_data: Arc<Mutex<HashMap<String, Arc<Link>>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let current;
        {
            let mut to_visit_val = to_visit.lock().unwrap();
            let mut active_count_val = active_count.lock().unwrap();
            if to_visit_val.is_empty() {
                if *active_count_val > 0 {
                    continue;
                } else {
                    break;
                }
            };
            current = to_visit_val.pop().unwrap();
            *active_count_val += 1;
            assert!(*active_count_val <= THREADS);
        }

        {
            let mut visited_val = visited.lock().unwrap();
            if visited_val.contains(&current) {
                let mut active_count_val = active_count.lock().unwrap();
                *active_count_val -= 1;
                continue;
            } else {
                visited_val.insert(current.to_owned());
            }
        }

        let state = url_status(&domain, &current);
        if let UrlState::Accessible(ref url) = state.clone() {
            if url.domain() == Some(&domain) {
                let new_links = fetch_all_links(&url, domain)?;

                let mut to_visit_val = to_visit.lock().unwrap();
                let mut link_data_val = link_data.lock().unwrap();
                for new_link in &new_links {
                    if let Some(href) = &new_link.href {
                        to_visit_val.push(href.clone());
                        link_data_val.insert(href.to_string(), new_link.clone());
                    }
                }
            }
        }

        {
            let mut active_count_val = active_count.lock().unwrap();
            *active_count_val -= 1;
            assert!(*active_count_val >= 0);
        }

        url_states.send(state).unwrap();
    }
    Ok(())
}

pub fn crawl(domain: &str, start_url: &Url) -> Crawler {
    let to_visit = Arc::new(Mutex::new(vec![start_url.to_string()]));
    let active_count = Arc::new(Mutex::new(0));
    let visited = Arc::new(Mutex::new(HashSet::new()));
    let link_data = Arc::new(Mutex::new(HashMap::new()));

    let (tx, rx) = channel();

    let crawler = Crawler {
        to_visit: to_visit.clone(),
        active_count: active_count.clone(),
        url_states: rx,
        link_data: link_data.clone(),
    };

    for _ in 0..THREADS {
        let domain = domain.to_owned();
        let to_visit = to_visit.clone();
        let visited = visited.clone();
        let active_count = active_count.clone();
        let link_data = link_data.clone();
        let tx = tx.clone();

        thread::spawn(move || {
            crawl_worker_thread(&domain, to_visit, visited, active_count, tx, link_data);
        });
    }

    crawler
}
