mod lib;
use async_std::task;
use clap::{App, Arg};
use lib::{BaseConf, Task};

fn parseConf(path: &str) -> BaseConf {
    use std::io::prelude::*;
    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            panic!("Could not find config file, using default!");
        }
    };
    let mut config_toml = String::new();
    file.read_to_string(&mut config_toml)
        .unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));
    let base_conf: BaseConf = toml::from_str(config_toml.as_str())
        .unwrap_or_else(|err| panic!("Error while parse config: [{}]", err));
    base_conf
}

fn main() {
    let matches = App::new("My Super Program")
        .version("1.0")
        .author("ZhengXin <zhnngxin@gmail.com>")
        .about("Does awesome things")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file to store")
                .required(false)
                .index(1),
        )
        .get_matches();

    let config = matches.value_of("config").unwrap_or("conf.toml");
    let mut base_conf = parseConf(config);
    if matches.is_present("OUTPUT") {
        base_conf.output = String::from(matches.value_of("OUTPUT").unwrap());
    }
    task::block_on(async {
        let mut task = match Task::new(&base_conf) {
            Ok(t) => t,
            Err(e) => panic!(e),
        };
        if let Err(e) = task.process().await {
            println!("{:?}", e);
        }
    });
}
