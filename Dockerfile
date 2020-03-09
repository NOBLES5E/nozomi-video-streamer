FROM ubuntu:rolling

WORKDIR /app

VOLUME /data

COPY . /app

RUN apt update && apt install -y ffmpeg fonts-noto-cjk

RUN apt-get update && apt-get install -y locales && rm -rf /var/lib/apt/lists/* \
    && localedef -i en_US -c -f UTF-8 -A /usr/share/locale/locale.alias en_US.UTF-8
ENV LANG en_US.utf8

CMD ./target/release/video-streamer-rs --serving-dir /data
