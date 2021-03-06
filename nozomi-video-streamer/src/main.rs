#![feature(async_closure)]
#![feature(str_strip)]
mod helper;
mod filters;

use anyhow::Result;
use std::io::Write;
use structopt::StructOpt;
use std::path::PathBuf;
use askama::Template;
use std::sync::{Arc, Mutex};
use warp::Filter;
use warp::filters::path::FullPath;
use chrono::{Local, Timelike};
use std::convert::Infallible;
use std::net::ToSocketAddrs;
use serde_derive::Deserialize;
use async_std::prelude::*;
use futures_util::StreamExt;
use tokio::process::Command;
use bytes::Buf;
use std::process::Stdio;
use tokio::time::Duration;

#[derive(Debug, StructOpt, Clone)]
#[structopt()]
struct Cli {
    #[structopt(long, default_value = "0.0.0.0:4000")]
    bind_address: String,
    #[structopt(long, default_value = ".")]
    serving_dir: String,
}

/// A struct representing a file/directory to be listed on the web UI.
struct DirectoryFile {
    filename: String,
    url: String,
}

/// A struct representing a directory on the web UI. Serving as the data model of template directory.html.
#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryTemplate {
    directory_path: String,
    files: Vec<DirectoryFile>,
}

/// Post parameters for the web api.
#[derive(Deserialize)]
struct PostParams {
    subtitle: Option<String>,
    bitrate: String,
    start_time: Option<String>,
    upload_subtitle_file: Option<String>,
}

/// Stream a file.
async fn file_to_stream(path: PathBuf, post_params: PostParams) -> Result<impl Stream<Item=Result<bytes::Bytes, std::io::Error>>> {
    let temp_dir = tempfile::tempdir()?;
    let mut child = {
        let mut subtitle = post_params.subtitle.clone();
        if let Some(subtitle) = &subtitle {
            if subtitle.ne("self") {
                anyhow::bail!("invalid subtitle mode")
            }
        }
        if let Some(upload_subtitle) = post_params.upload_subtitle_file {
            let temp_sub_path = temp_dir.path().join("upload.ass");
            let upload_subtitle = base64::decode(&upload_subtitle)?;
            std::fs::write(&temp_sub_path, upload_subtitle)?;
            subtitle = Some((&temp_sub_path).to_str().ok_or(anyhow::anyhow!("path to str failed"))?.to_string());
        };
        let bitrate = &post_params.bitrate[..];
        let start_time = &post_params.start_time.unwrap_or("00:00:00".parse()?);
        match &subtitle {
            None => Command::new("ffmpeg")
                .arg("-ss").arg(start_time)
                .arg("-i")
                .arg(path.to_str().unwrap())
                .arg("-b:v").arg(bitrate)
                .arg("-cpu-used").arg("-8")
                .arg("-deadline").arg("realtime")
                .arg("-vcodec").arg("libx264")
                .arg("-acodec").arg("aac")
                .arg("-framerate").arg("15")
                .arg("-f").arg("flv").arg("-")
                .stdout(Stdio::piped())
                .spawn().expect("cannot spawn command"),
            Some(subpath) => {
                let temp_sub_path = temp_dir.path().join("out.ass");
                let subpath = match subpath.as_str() {
                    "self" => path.to_str().ok_or(anyhow::anyhow!("to str failed"))?,
                    _ => subpath.as_str()
                };
                if !Command::new("ffmpeg")
                    .arg("-ss").arg(start_time)
                    .arg("-i").arg(subpath)
                    .arg(temp_sub_path.to_str().ok_or(anyhow::anyhow!("path to str failed"))?).spawn()?.await?.success() {
                    anyhow::bail!("cannot convert subtitle");
                };
                Command::new("ffmpeg")
                    .arg("-ss").arg(start_time)
                    .arg("-i")
                    .arg(path.to_str().unwrap())
                    .arg("-vf").arg(helper::ffmpeg_filtergraph_escaping(format!("subtitles={}", temp_sub_path.to_str().unwrap()).as_str()))
                    .arg("-b:v").arg(bitrate)
                    .arg("-cpu-used").arg("-8")
                    .arg("-deadline").arg("realtime")
                    .arg("-vcodec").arg("libx264")
                    .arg("-acodec").arg("aac")
                    .arg("-framerate").arg("15")
                    .arg("-f").arg("flv").arg("-")
                    .stdout(Stdio::piped())
                    .spawn().expect("cannot spawn command")
            }
        }
    };
    let stdout = child.stdout.take().expect("cannot read child stdout");
    let reader = tokio_util::codec::FramedRead::new(stdout, tokio_util::codec::BytesCodec::new());
    let result = reader.map(|x| { x.map(|mut y: bytes::BytesMut| bytes::Bytes::from(y.to_bytes())) });
    let _: tokio::task::JoinHandle<_> = tokio::spawn(
        async {
            child.await.expect("child process encountered an error")
        }
    );
    tokio::time::delay_for(Duration::from_secs(5)).await;
    Ok(result)
}

/// Show a directory on the web UI.
async fn serve_dir(path: FullPath, data: SharedAppData) -> Result<impl warp::Reply, Infallible> {
    let path: PathBuf = percent_encoding::percent_decode_str(&path.as_str()[1..]).decode_utf8().expect("cannot decode url").parse()?;
    log::info!("path: {:?}", path);
    let realpath = data.lock().unwrap().serving_dir.join(&path);
    log::info!("realpath: {:?}", realpath);
    let mut directory_path = path.to_str().unwrap().to_owned();
    if directory_path == "" {
        directory_path = "root directory".to_string();
    }
    let mut response = DirectoryTemplate {
        directory_path: directory_path,
        files: std::fs::read_dir(&realpath).expect("cannot read directory").into_iter().map(
            |entry| {
                let filename = entry.expect("cannot read file").file_name().to_str().unwrap().to_owned();
                log::info!("filename: {:?}", filename);
                let url = path.join(filename.clone()).to_str().unwrap().to_string();
                DirectoryFile {
                    filename: filename.clone(),
                    url: "/".to_string() + url.strip_prefix("/").unwrap_or(url.as_str()),
                }
            }
        ).collect(),
    };
    response.files.sort_by(|a, b| { a.filename.cmp(&b.filename) });
    let response = response.render().unwrap();
    return Ok(hyper::Response::builder().status(hyper::StatusCode::OK).body(response).unwrap());
}

/// API: serve a file.
async fn serve_file(path: FullPath, data: SharedAppData) -> Result<impl warp::Reply, Infallible> {
    let path: PathBuf = percent_encoding::percent_decode_str(&path.as_str()[1..]).decode_utf8().expect("cannot decode url").parse()?;
    let realpath = data.lock().unwrap().serving_dir.join(&path);
    let file = async_std::fs::File::open(realpath).await.expect("cannot open file");
    Ok(hyper::Response::builder().status(hyper::StatusCode::OK).body(hyper::Body::wrap_stream(
        file.bytes().map(|x| { x.map(|y| bytes::Bytes::from(vec![y])) })
    )).expect("cannot build response"))
}

/// API: serve a file with on the fly transcoding.
async fn serve_convert_file(path: FullPath, data: SharedAppData, params: PostParams) -> Result<impl warp::Reply, Infallible> {
    log::info!("serving converted file");
    let path: PathBuf = percent_encoding::percent_decode_str(&path.as_str()[1..]).decode_utf8().expect("cannot decode url").parse()?;
    let realpath = data.lock().unwrap().serving_dir.join(&path);
    Ok(hyper::Response::builder().status(hyper::StatusCode::OK)
        .body(hyper::Body::wrap_stream(file_to_stream(realpath, params).await.expect("cannot convert file to stream"))).unwrap()
    )
}

/// Shared global data for the web server
pub struct AppData {
    serving_dir: PathBuf
}

type SharedAppData = Arc<Mutex<AppData>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Cli = Cli::from_args();
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
        }).init();

    let data = Arc::new(
        Mutex::new(
            AppData {
                serving_dir: args.serving_dir.parse()?
            }
        )
    );
    let dir_route = warp::path::full()
        .and(filters::is_dir(data.clone()))
        .and(filters::with_shared_info(data.clone()))
        .and_then(serve_dir);
    let file_route = warp::path::full()
        .and(filters::is_file(data.clone()))
        .and(filters::with_shared_info(data.clone()))
        .and_then(serve_file);
    let convert_file_route = warp::path::full()
        .and(warp::post())
        .and(filters::is_file(data.clone()))
        .and(filters::with_shared_info(data.clone()))
        .and(warp::body::json())
        .and_then(serve_convert_file);
    warp::serve(dir_route.or(convert_file_route).or(file_route)).run(args.bind_address.to_socket_addrs()?.next().expect("cannot parse bind addr")).await;
    Ok(())
}
