use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::Deserialize;
#[derive(Deserialize, Debug)]
pub struct BaseConf {
    pub base: String,
    pub url_list: Vec<String>,
    pub output: String,
    pub title: String,
    pub content: String,
    pub proxy: String,
}
#[derive(Default, Debug)]
pub struct Task {
    is_running: AtomicBool,
    current: AtomicUsize,
    base_url: String,
    url_list: Vec<String>,
    output: String,
    title_selector: String,
    content_selector: String,
}

#[derive(Debug)]
pub struct ErrorWithStr {
    details: String,
}

impl ErrorWithStr {
    pub fn new(msg: &str) -> Self {
        Self {
            details: msg.to_string(),
        }
    }
}

impl std::fmt::Display for ErrorWithStr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for ErrorWithStr {
    fn description(&self) -> &str {
        &self.details
    }
}

impl Task {
    pub fn new(conf: &'_ BaseConf) -> Result<Self, String> {
        let mut t = Task {
            is_running: AtomicBool::new(false),
            base_url: conf.base.clone(),
            output: conf.output.clone(),
            ..Default::default()
        };
        t.url_list = conf.url_list.iter().map(|s| s.clone()).collect();
        if conf.title.len() > 0 {
            t.title_selector = conf.title.clone();
        }
        if conf.content.len() > 0 {
            t.content_selector = conf.content.clone();
        } else {
            return Err(String::from("content is expected"));
        }
        Ok(t)
    }
    fn is_url_valid(&self, next: &'_ String) -> bool {
        if !next.ends_with(".html") {
            return false;
        }
        true
    }
    pub fn stop(&mut self) {
        *self.is_running.get_mut() = false;
    }
    pub async fn process(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>> {
        use async_std::prelude::*;
        *self.is_running.get_mut() = true;
        *self.current.get_mut() = 0;
        let mut output = async_std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(self.output.as_str())
            .await?;
        let mut url: String;
        let sty = indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-");
        let pb: indicatif::ProgressBar = indicatif::ProgressBar::new(self.url_list.len() as u64);
        pb.set_style(sty);
        loop {
            if !self.is_running.load(Ordering::SeqCst) {
                return Ok(());
            }
            let current = self.current.load(Ordering::SeqCst);
            if let Some(s) = self.url_list.get(current) {
                url = self.base_url.clone();
                url.push_str(s.as_str());
                pb.set_message(&format!("item {:?}", s));
                pb.inc(1);
            } else {
                return Err(Box::new(ErrorWithStr::new(
                    "index out of board for current",
                )));
            }
            let mut res = surf::get(&url).await?;
            let document = scraper::Html::parse_document(&res.body_string().await?);
            if !self.title_selector.is_empty() {
                let _title_selector = scraper::Selector::parse(&self.title_selector).unwrap();
                let title = document.select(&_title_selector).next().unwrap();
                for s in title.text() {
                    output.write_all(s.as_bytes()).await?;
                }
                output.write_all(&['\n' as u8]).await?;
            }
            {
                let _content_selector = scraper::Selector::parse(&self.content_selector).unwrap();
                if let Some(content) = document.select(&_content_selector).next() {
                    for s in content.text() {
                        output.write_all(s.as_bytes()).await?;
                    }
                    output.write_all(&['\n' as u8]).await?;
                } else {
                    return Err(Box::new(ErrorWithStr::new("no content found")));
                }
            }
            {
                *self.current.get_mut() += 1;
                if self.current.load(Ordering::SeqCst) >= self.url_list.len() {
                    *self.is_running.get_mut() = false;
                    pb.finish_with_message("done");
                    return Ok(());
                }
            }
        }
    }
}
