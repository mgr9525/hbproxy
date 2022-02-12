
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release

scp -P 122 target/release/hbproxy mgr@yldown.jazpan.com:~/website/static/rust/linux64
#scp -P 122 target/debug/hbproxy mgr@yldown.jazpan.com:~/website/static/rust/linux64

sudo cp target/release/hbproxy /usr/local/bin/
