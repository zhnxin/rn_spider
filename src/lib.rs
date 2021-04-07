use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::Deserialize;
#[derive(Default, Deserialize, Debug)]
pub struct BaseConf {
    pub base: String,
    pub url_list: Vec<String>,
    pub title: String,
    pub content: String,
    pub next: String,
    pub next_regexp: String,
}
#[derive(Default, Debug)]
pub struct Task {
    base: BaseConf,
    is_running: AtomicBool,
    output: String,
    current: AtomicUsize,
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
    pub fn new(conf: BaseConf, ouput: String) -> Result<Self, String> {
        let t = Task {
            is_running: AtomicBool::new(false),
            base: conf,
            output: ouput,
            ..Default::default()
        };
        if t.base.content.is_empty() {
            return Err(String::from("content is expected"));
        }
        Ok(t)
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
        let pb: indicatif::ProgressBar =
            indicatif::ProgressBar::new(self.base.url_list.len() as u64);
        pb.set_style(sty);
        let _title_selector: Option<scraper::Selector> = if !self.base.title.is_empty() {
            Some(scraper::Selector::parse(&self.base.title).unwrap())
        } else {
            None
        };
        let _content_selector = scraper::Selector::parse(&self.base.content).unwrap();
        let _next_selector: Option<scraper::Selector> = if !self.base.next.is_empty() {
            Some(scraper::Selector::parse(&self.base.next).unwrap())
        } else {
            None
        };

        let _next_pattern: Option<regex::Regex> = if !self.base.next_regexp.is_empty() {
            Some(regex::Regex::new(self.base.next_regexp.as_ref()).unwrap())
        } else {
            None
        };

        loop {
            if !self.is_running.load(Ordering::SeqCst) {
                return Ok(());
            }
            let current = self.current.load(Ordering::SeqCst);
            if let Some(s) = self.base.url_list.get(current) {
                url = self.base.base.clone();
                url.push_str(s.as_str());
                pb.set_message(&format!("item {:?}", s));
            } else {
                return Err(Box::new(ErrorWithStr::new(
                    "index out of board for current",
                )));
            }
            let mut res = surf::get(&url).await?;
            let document = scraper::Html::parse_document(&res.body_string().await?);
            if let Some(selector) = _title_selector.as_ref() {
                let title = document.select(selector).next().unwrap();
                for s in title.text() {
                    output.write_all(s.as_bytes()).await?;
                }
                output.write_all(&['\n' as u8]).await?;
            }
            {
                if let Some(content) = document.select(&_content_selector).next() {
                    for s in content.text() {
                        output.write_all(s.as_bytes()).await?;
                    }
                    output.write_all(&['\n' as u8]).await?;
                } else {
                    return Err(Box::new(ErrorWithStr::new("no content found")));
                }
            }
            if let Some(selector) = _next_selector.as_ref() {
                if let Some(next) = document.select(selector).next() {
                    if let Some(href) = next.value().attr("href") {
                        if let Some(pattern) = _next_pattern.as_ref() {
                            if pattern.is_match(href) {
                                self.base.url_list[current] = String::from(href);
                                continue;
                            }
                        } else {
                            self.base.url_list[current] = String::from(href);
                            continue;
                        }
                    }
                }
            }
            {
                *self.current.get_mut() += 1;
                pb.inc(1);
                if self.current.load(Ordering::SeqCst) >= self.base.url_list.len() {
                    *self.is_running.get_mut() = false;
                    pb.finish_with_message(&format!("done: {}", self.base.url_list[current]));
                    return Ok(());
                }
            }
        }
    }
}
