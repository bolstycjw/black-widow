extern crate serde;
extern crate serde_json;
extern crate markup5ever_rcdom as rcdom;
extern crate reqwest;
extern crate tokio;
extern crate url;

mod crawler;
mod fetch;
mod parse;

use std::env;
use std::time::Instant;
use fetch::UrlState;
use std::io::Write;
use std::io::stdout;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 {
        let url_string = &args[1];
        let start_url = Url::parse(url_string)?;
        let domain = start_url
            .domain()
            .expect("I can't find a domain in your URL");

        let mut success_count = 0;
        let mut fail_count = 0;

        let now = Instant::now();
        for url_state in crawler::crawl(&domain, &start_url) {
            match url_state {
                (UrlState::Accessible(_), _) => {
                    success_count += 1;
                }
                (status, maybe_link) => {
                    fail_count += 1;
                    match status {
                        UrlState::BadStatus(_, _) => {
                            match maybe_link {
                                 Some(link) =>  println!("{}", serde_json::to_string(&*link)?),
                                 None => {}
                            }
                        },
                        _ => {}
                    }
                    println!("{}", status);
                }
            }

            print!("Succeeded: {} Failed: {}\r", success_count, fail_count);
            stdout().flush().unwrap();
        }
        let elapsed = now.elapsed();
        println!("Elasped: {:.2?}", elapsed);
        println!("Total crawled: {}", success_count + fail_count);
    } else {
        println!("Please provide a URL.");
    }
    Ok(())
}
