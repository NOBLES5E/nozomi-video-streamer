/// Various filters to build the Restful API.
use crate::SharedAppData;
use warp::Filter;
use warp::filters::path::FullPath;
use std::path::PathBuf;
use warp::filters::BoxedFilter;

pub fn with_shared_info(db: SharedAppData) -> BoxedFilter<(SharedAppData, )> {
    warp::any().map(move || {
        db.clone()
    }).boxed()
}

pub fn is_dir(db: SharedAppData) -> BoxedFilter<()> {
    warp::path::full().and(with_shared_info(db)).and_then(
        async move |path: FullPath, data: SharedAppData| {
            let path: PathBuf = percent_encoding::percent_decode_str(&path.as_str()[1..]).decode_utf8().expect("cannot decode url").parse()?;
            let realpath = data.lock().unwrap().serving_dir.join(&path);
            log::info!("real dir path {:?}", realpath);
            match realpath.is_dir() {
                true => Ok(()),
                false => Err(warp::reject::reject()),
            }
        }
    ).untuple_one().boxed()
}

pub fn is_file(db: SharedAppData) -> BoxedFilter<()> {
    warp::path::full().and(with_shared_info(db)).and_then(
        async move |path: FullPath, data: SharedAppData| {
            let path: PathBuf = percent_encoding::percent_decode_str(&path.as_str()[1..]).decode_utf8().expect("cannot decode url").parse()?;
            let realpath = data.lock().unwrap().serving_dir.join(&path);
            log::info!("real file path {:?}", realpath);
            match realpath.is_file() {
                true => Ok(()),
                false => Err(warp::reject::reject()),
            }
        }
    ).untuple_one().boxed()
}

