
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target x86_64-apple-darwin

SERVERS=yldown.jazpan.com
#SERVERS=gy.ruisgo.com
# SERVERS=linux.1ydt.cn


scp -P 122 target/x86_64-apple-darwin/release/hbproxy mgr@$SERVERS:~/website/static/rust/darwin64
