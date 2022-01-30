
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build

target/debug/hbproxy --debug node join local
