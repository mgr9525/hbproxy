
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target x86_64-pc-windows-gnu

SERVERS=yldown.jazpan.com
#SERVERS=gy.ruisgo.com
# SERVERS=linux.1ydt.cn


scp -P 122 target/x86_64-pc-windows-gnu/release/hbproxy.exe mgr@$SERVERS:~/website/static/rust/win64
