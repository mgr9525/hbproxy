
export RUST_BACKTRACE=1
# export RUSTFLAGS="-C target-feature=+crt-static"
cargo build

# target/debug/hbproxy --debug server
target/debug/hbproxy --debug -k 123456 server
