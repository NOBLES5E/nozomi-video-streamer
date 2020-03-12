## Nozomi Video Streamer

A dead simple on demand video streaming service written in rust.

The service serves a directory, and (optionally) transcode the media on the fly.

Typical use cases include

* Downloading videos on a server with large internet throughput. Then view the video on your laptop (starting from arbitrary position of the video), optionally with lower quality to save bandwidth and get more smooth playing experience.

### Getting started

Download the binary from release page.

On server 

```
./video-streamer-rs --serving-dir /data
```

On client now you can play video with (for example)

```
http get 'https://your-domain.com/video-name.mp4' | mpv -
```
