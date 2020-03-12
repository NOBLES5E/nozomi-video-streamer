![](./logo.png | width=100)

A dead simple personal on demand video streaming service written in [Rust](https://www.rust-lang.org/) based on [warp](https://github.com/seanmonstar/warp) and [async/await](https://github.com/rust-lang/rust/issues/50547).

The service serves a directory, and (optionally) transcode the media on the fly.

Typical use cases include

* Downloading videos on a server with large internet throughput. Then view the video on your laptop (starting from arbitrary position of the video), optionally with lower quality to save bandwidth and get more smooth playing experience.
* Combine with another service like qBittorrent, which downloads submitted links to a specified directory, which can be served by Nozomi Video Streamer.

### Getting started

Download the binary from release page. Ensure you have `ffmpeg` installed on your server.

On server 

```
./nozomi-video-streamer --help
./nozomi-video-streamer --serving-dir /data
```

On client now you can play video with (for example)

```
http get 'https://your-domain.com:4000/video-name.mp4' | mpv -
```
