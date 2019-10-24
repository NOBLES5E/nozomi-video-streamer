use anyhow::Result;
use structopt::StructOpt;
use log;
use actix_web::{HttpServer, web, HttpRequest, App, HttpResponse, Responder, Either};
use std::path::PathBuf;
use std::sync::Mutex;
use askama::{Template, Error};
use futures::{Stream, IntoStream};
use actix_web::body::{Body, BodyStream};
use tokio_process::{ChildStdout, CommandExt};
use std::process::{Stdio, Command};
use tokio::codec::{FramedRead, BytesCodec};
use std::io::Bytes;
use std::iter::once;
use tokio::prelude::Future;
use serde::Deserialize;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "video-streamer-rs")]
struct Cli {
    #[structopt(long, default_value = "0.0.0.0:4000")]
    bind_address: String,
    #[structopt(long, default_value = ".")]
    serving_dir: String,
}

struct DirectoryFile {
    filename: String,
    url: String,
}

#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryTemplate {
    directory_path: String,
    files: Vec<DirectoryFile>,
}

struct SiteData {
    serving_dir: PathBuf,
}

/// See https://ffmpeg.org/ffmpeg-filters.html#toc-Notes-on-filtergraph-escaping
fn ffmpeg_filtergraph_escaping(raw_string: &str) -> String {
    // first level
    let result = raw_string.replace(r#"'"#, r#"\'"#);
    let result = result.replace(r#":"#, r#"\:"#);
    // second levresult
    let result = result.replace(r#"\"#, r#"\\"#);
    let result = result.replace(r#"'"#, r#"\'"#);
    let result = result.replace(r#"["#, r#"\["#);
    let result = result.replace(r#"]"#, r#"\]"#);
    let result = result.replace(r#","#, r#"\,"#);
    let result = result.replace(r#";"#, r#"\;"#);
    log::info!("ffmpeg filter graph {:?}", result);
    return result;
}

fn file_to_stream(path: PathBuf, mode: &str, bitrate: &str) -> Result<impl Stream<Item=bytes::Bytes, Error=impl actix_http::error::ResponseError>> {
    let mut child = match mode {
        "convert" => Command::new("ffmpeg")
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
            .spawn_async().unwrap(),
        "convert_self_subtitle" => Command::new("ffmpeg")
            .arg("-i")
            .arg(path.to_str().unwrap())
            .arg("-vf").arg(ffmpeg_filtergraph_escaping(format!("subtitles={}", path.to_str().unwrap()).as_str()))
            .arg("-b:v").arg(bitrate)
            .arg("-cpu-used").arg("-8")
            .arg("-deadline").arg("realtime")
            .arg("-vcodec").arg("libx264")
            .arg("-acodec").arg("aac")
            .arg("-framerate").arg("15")
            .arg("-f").arg("flv").arg("-")
            .stdout(Stdio::piped())
            .spawn_async().unwrap(),
        "" => Command::new("cat")
            .arg(path.to_str().unwrap())
            .stdout(Stdio::piped())
            .spawn_async().unwrap(),
        _ => { return Err(anyhow::anyhow!("invalid mode type")); }
    };
    let stdout = child.stdout().take().unwrap();
    let mut reader = FramedRead::new(stdout, BytesCodec::new());
    let result = reader.map(|mut x| { bytes::Bytes::from(x) });
    tokio::spawn(child.map(|status| {}).map_err(|e| { log::error!("error {:?}", e) }));
    return Ok(result);
}


#[derive(Deserialize)]
struct QueryParams {
    mode: String,
    bitrate: String,
}

fn index(req: HttpRequest, data: web::Data<Mutex<SiteData>>, query_params: Option<web::Query<QueryParams>>) -> HttpResponse {
    let mut path: PathBuf = req.match_info().query("filename").parse().unwrap();
    if path.to_str().unwrap().len() == 0 {
        path = PathBuf::from(".");
    }
    let realpath = data.lock().unwrap().serving_dir.join(&path);
    if realpath.is_dir() {
        let mut directory_path = path.to_str().unwrap().to_owned();
        if directory_path == "." {
            directory_path = "root directory".parse().unwrap();
        }
        let mut response = DirectoryTemplate {
            directory_path: directory_path,
            files: std::fs::read_dir(&realpath).unwrap().into_iter().map(
                |entry| {
                    let filename = entry.unwrap().file_name().to_str().unwrap().to_owned();
                    DirectoryFile {
                        filename: filename.clone(),
                        url: path.join(filename).to_str().unwrap().to_owned(),
                    }
                }
            ).collect(),
        };
        response.files.sort_by(|a, b| { a.filename.cmp(&b.filename) });
        let response = response.render().unwrap();
        return HttpResponse::Ok().body(response);
    } else if realpath.is_file() {
//        let mut child = Command::new("cat").arg(path.to_str().unwrap()).stdout(Stdio::piped())
//            .spawn_async().unwrap();
        let result = file_to_stream(realpath,
                                    query_params.as_ref().unwrap().mode.as_ref(),
                                    query_params.as_ref().unwrap().bitrate.as_ref()).expect("cannot convert file to byte stream");
        return HttpResponse::Ok().content_type("application/octet-stream").streaming(result);
    } else {
        return HttpResponse::BadRequest().body("no such file or directory");
    }
}

fn main() -> Result<()> {
    let args: Cli = Cli::from_args();
    fern::Dispatch::new()
        .chain(std::io::stderr())
        .level(log::LevelFilter::Info)
        .level_for("video-streamer-rs", log::LevelFilter::Debug)
        .apply().expect("cannot initialize fern logger");

    let data = actix_web::web::Data::new(
        Mutex::new(
            SiteData {
                serving_dir: args.serving_dir.clone().parse().unwrap()
            }
        )
    );

    let serving_dir = args.serving_dir.clone();
    HttpServer::new(move || {
        App::new()
            .register_data(data.clone())
            .route("/{filename:.*}", web::get().to(index))
    }).bind(args.bind_address).unwrap().run().unwrap();
    Ok(())
}
