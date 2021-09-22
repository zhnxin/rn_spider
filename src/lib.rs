use rand::prelude::*;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[derive(Default, Deserialize, Debug)]
pub struct BaseConf {
    pub base: String,
    pub url_list: Vec<String>,
    #[serde(default)]
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub next: String,
    #[serde(default)]
    pub next_regexp: String,
    #[serde(default)]
    pub sub: String,
    #[serde(default)]
    pub sub_regexp: String,
    #[serde(default)]
    pub encoding: String,
    // skip the current page, and start to store next page if existed or sub page
    #[serde(default)]
    pub is_expired_next: bool,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub random_sleep_millis: u64,
    #[serde(default)]
    pub is_inner_html: bool,
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
        let mut t = Task {
            is_running: AtomicBool::new(false),
            base: conf,
            output: ouput,
            ..Default::default()
        };
        if t.base.content.is_empty() {
            return Err(String::from("content is expected"));
        }
        if t.base.encoding.is_empty() {
            t.base.encoding = std::string::String::from("utf-8");
        }
        if t.base.agent.is_empty() {
            t.base.agent = std::string::String::from("Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:90.0) Gecko/20100101 Firefox/90.0")
        }
        Ok(t)
    }
    // pub fn stop(&mut self) {
    //     *self.is_running.get_mut() = false;
    // }
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
        let encoding_format = encoding::label::encoding_from_whatwg_label(&self.base.encoding)
            .expect("unknow encoding");
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
        let _sub_selector: Option<scraper::Selector> = if !self.base.sub.is_empty() {
            Some(scraper::Selector::parse(&self.base.sub).unwrap())
        } else {
            None
        };

        let _sub_pattern: Option<regex::Regex> = if !self.base.sub_regexp.is_empty() {
            Some(regex::Regex::new(self.base.sub_regexp.as_ref()).unwrap())
        } else {
            None
        };
        let mut item_count = 0_usize;
        let mut rng = rand::thread_rng();
        let mut sub_url_list: Vec<String> = Vec::new();
        let mut item: String;
        let mut is_expired_next = self.base.is_expired_next;

        loop {
            if !self.is_running.load(Ordering::SeqCst) {
                return Ok(());
            }
            let current = self.current.load(Ordering::SeqCst);
            // 存在子页面
            if let Some(s) = sub_url_list.last() {
                item = String::from(s);
            } else if let Some(s) = self.base.url_list.get(current) {
                item = String::from(s);
            } else {
                return Err(Box::new(ErrorWithStr::new(
                    "index out of board for current",
                )));
            }
            url = self.base.base.clone();
            url.push_str(item.as_str());
            pb.set_message(&format!("item({:04}) {:?}", item_count, &item));
            if item_count > 0 && self.base.random_sleep_millis > 0 {
                async_std::task::sleep(std::time::Duration::from_millis(
                    rng.gen::<u64>() % self.base.random_sleep_millis,
                ))
                .await;
            }
            item_count += 1;
            let document = scraper::Html::parse_document(
                encoding_format
                    .decode(
                        match surf::get(&url)
                            .header("User-Agent", &self.base.agent)
                            .recv_bytes()
                            .await
                        {
                            Ok(res) => res,
                            Err(e) => {
                                return Err(Box::new(ErrorWithStr::new(&format!(
                                    "item({:04}) {:?}: {:?}",
                                    item_count, &item, e
                                ))))
                            }
                        }
                        .as_slice(),
                        encoding::types::DecoderTrap::Ignore,
                    )
                    .unwrap()
                    .as_str(),
            );
            if !is_expired_next {
                if let Some(selector) = _title_selector.as_ref() {
                    let title = document.select(selector).next().unwrap();
                    for s in title.text() {
                        output.write_all(s.as_bytes()).await?;
                    }
                    output.write_all(&['\n' as u8]).await?;
                }
                {
                    if let Some(content) = document.select(&_content_selector).next() {
                        if self.base.is_inner_html {
                            output.write_all(content.html().as_bytes()).await?;
                            output.write_all(&['\n' as u8]).await?;
                        } else {
                            for s in content.text() {
                                output.write_all(s.as_bytes()).await?;
                            }
                            output.write_all(&['\n' as u8]).await?;
                        }
                    } else {
                        return Err(Box::new(ErrorWithStr::new("no content found")));
                    }
                }
            } else {
                is_expired_next = false;
            }
            // 不存在子页面且有配置下一页面selector
            if sub_url_list.is_empty() {
                if let Some(selector) = _sub_selector.as_ref() {
                    for _sub in document.select(selector) {
                        if let Some(href) = _sub.value().attr("href") {
                            if let Some(pattern) = _sub_pattern.as_ref() {
                                if pattern.is_match(href) {
                                    sub_url_list.push(String::from(href));
                                }
                            } else {
                                sub_url_list.push(String::from(href));
                            }
                        }
                    }
                    sub_url_list.reverse();
                }
            } else {
                sub_url_list.pop();
            }
            if sub_url_list.is_empty() {
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
