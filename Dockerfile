FROM rust:latest

WORKDIR /app

VOLUME /data

COPY . /app

RUN cargo build --release && cp target/release/video-streamer-rs /bin

RUN apt update && apt install -y ffmpeg

CMD video-streamer-rs --serving-dir /data
