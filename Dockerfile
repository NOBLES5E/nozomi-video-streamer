FROM ubuntu:latest

WORKDIR /app

VOLUME /data

COPY . /app

RUN apt update && apt install -y ffmpeg fonts-noto-cjk

CMD ./target/release/video-streamer-rs --serving-dir /data
