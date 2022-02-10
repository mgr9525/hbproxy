
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target i686-pc-windows-gnu

SERVERS=yldown.jazpan.com
#SERVERS=gy.ruisgo.com
# SERVERS=linux.1ydt.cn


scp -P 122 target/i686-pc-windows-gnu/release/hbproxy mgr@$SERVERS:~/website/static/rust/win32
