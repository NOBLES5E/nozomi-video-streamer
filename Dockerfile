FROM ubuntu:latest

WORKDIR /app

VOLUME /data

COPY . /app

RUN apt update && apt install -y ffmpeg fonts-noto-cjk

RUN sed -i -e 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen && locale-gen
ENV LANG en_US.UTF-8  
ENV LANGUAGE en_US:en  
ENV LC_ALL en_US.UTF-8     

CMD ./target/release/video-streamer-rs --serving-dir /data
