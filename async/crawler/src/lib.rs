#![forbid(unsafe_code)]

use futures::future::select_all;
use linkify::{LinkFinder, LinkKind};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct Config {
    pub concurrent_requests: Option<usize>,
}

#[derive(Debug)]
pub struct Page {
    pub url: String,
    pub body: String,
}

pub struct Crawler {
    config: Config,
}

impl Crawler {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn run(&mut self, site: String) -> Receiver<Page> {
        let limit = self.config.concurrent_requests.unwrap_or(1);
        let (tx, rx) = channel(limit);
        let mut vec_futures = Vec::with_capacity(limit);
        let queue = Arc::new(Mutex::new(VecDeque::new()));

        let visited = Arc::new(Mutex::new(HashSet::new()));
        vec_futures.push(Box::pin(Crawler::foo(
            site,
            tx.clone(),
            Arc::clone(&visited),
            Arc::clone(&queue),
        )));

        tokio::spawn(async move {
            loop {
                let (_, _, mut remaining) = select_all(vec_futures).await;
                let mut locked_queue = queue.lock().unwrap();
                if remaining.is_empty() && locked_queue.is_empty() {
                    break;
                }
                if locked_queue.is_empty() {
                    vec_futures = remaining;
                    continue;
                }
                while remaining.len() < limit && !locked_queue.is_empty() {
                    remaining.push(Box::pin(Crawler::foo(
                        locked_queue.pop_front().unwrap(),
                        tx.clone(),
                        Arc::clone(&visited),
                        Arc::clone(&queue),
                    )));
                }
                vec_futures = remaining;
            }
        });

        rx
    }

    async fn foo(
        site: String,
        tx: Sender<Page>,
        visited: Arc<Mutex<HashSet<String>>>,
        bfs: Arc<Mutex<VecDeque<String>>>,
    ) {
        {
            let mut locked_set = visited.lock().unwrap();
            if locked_set.contains(&site) {
                return;
            } else {
                locked_set.insert(site.clone());
            }
        }
        let body = reqwest::get(site.as_str())
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);
        let links: Vec<String> = finder
            .links(body.as_str())
            .map(|l| l.as_str().to_string())
            .collect();
        let page = Page { url: site, body };
        tx.send(page).await.unwrap();
        for link in links {
            bfs.lock().unwrap().push_back(link);
        }
    }
}
