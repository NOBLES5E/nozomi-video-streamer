FROM rust:latest

WORKDIR /app

VOLUME /data

COPY . /app

RUN cargo build && cp target/debug/video-streamer-rs /bin
# ADD ./target/release/video-streamer-rs /bin/video-streamer-rs

RUN apt update && apt install -y ffmpeg

CMD video-streamer-rs --serving-dir /data
