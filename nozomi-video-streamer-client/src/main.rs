use structopt::StructOpt;
use anyhow::Result;
use reqwest::Client;
use std::io::Write;
use std::process::Command;
use serde_derive::Serialize;
use chrono::{Duration, Timelike, Local};

/// API post parameter definition
#[derive(Serialize)]
struct PostParams {
    subtitle: Option<String>,
    bitrate: String,
    start_time: Option<String>,
    upload_subtitle_file: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt()]
struct Cli {
    /// read from command line or clipboard
    #[structopt(long)]
    url: Option<String>,
    #[structopt(long)]
    subtitle: Option<String>,
    #[structopt(long, default_value = "1M")]
    bitrate: String,
    #[structopt(long)]
    start_time: Option<Vec<String>>,
    #[structopt(long)]
    sub_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env("LOG_LEVEL")
        .format(|buf, record| {
            writeln!(buf,
                     "{} [{}] [{}:{}] - {}",
                     Local::now().format("%Y-%m-%dT%H:%M:%S"),
                     record.level(),
                     record.file().unwrap_or(""),
                     record.line().unwrap_or(0),
                     record.args()
            )
        })
        .init();

    let args: Cli = Cli::from_args();

    let client = Client::builder().build()?;

    let url = match args.url {
        Some(x) => x,
        None => {
            let stdout = Command::new("xclip").arg("-o").stdout(std::process::Stdio::piped()).spawn()?.wait_with_output()?.stdout;
            String::from_utf8(stdout)?
        }
    };

    eprintln!("playing: {}", url);

    // parse start time argument
    let mut start_time = chrono::NaiveTime::from_hms(0, 0, 0);
    if let Some(start_times) = args.start_time {
        for t in start_times {
            let delta = chrono::NaiveTime::parse_from_str(&t, "%H:%M:%S")?;
            let delta = Duration::seconds(delta.num_seconds_from_midnight() as i64);
            start_time += delta;
        }
    }

    let post_params = PostParams {
        subtitle: args.subtitle,
        bitrate: args.bitrate,
        start_time: Some(start_time.format("%H:%M:%S").to_string()),
        upload_subtitle_file: args.sub_file.map(
            |f| {
                base64::encode(&std::fs::read(f).unwrap())
            }
        )
    };


    let mut resp: reqwest::Response = client
        .post((url).as_str())
        .body(serde_json::to_string(&post_params)?)
        .send().await?;
    while let Some(chunk) = resp.chunk().await? {
        std::io::stdout().write_all(&chunk[..])?;
    }
    Ok(())
}
