FROM mgr9525/ubuntu:21-ali
RUN mkdir -p /data
COPY target/release/hbproxy /usr/local/bin
WORKDIR /data
ENTRYPOINT ["/usr/local/bin/hbproxy"]
