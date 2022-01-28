
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release

# SERVERS=yldown.jazpan.com
SERVERS=gy.ruisgo.com
# SERVERS=linux.1ydt.cn


#scp -P 122 target/release/server mgr@$SERVERS:~/temp/
