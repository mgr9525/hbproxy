
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target aarch64-unknown-linux-gnu

SERVERS=yldown.jazpan.com
#SERVERS=gy.ruisgo.com
# SERVERS=linux.1ydt.cn


scp -P 122 target/aarch64-unknown-linux-gnu/release/hbproxy mgr@$SERVERS:~/website/static/rust/aarch
