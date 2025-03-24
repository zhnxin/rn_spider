use rn_spider::{BaseConf, Task};
use clap::Parser;

/// act like a spider
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args{
    /// a custom config file
    #[arg(short,default_value = "config.toml")]
    config:String,
    /// the output file to store
    output:String,
}

fn parse_conf(path: &str) -> BaseConf {
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

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let base_conf = parse_conf(args.config.as_str());
    let mut task = match Task::new(base_conf,args.output)
        {
            Ok(t) => t,
            Err(e) => panic!("{}", e),
        };
    if let Err(e) = task.process().await {
        println!("{}", e);
    }
}
