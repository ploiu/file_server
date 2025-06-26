rm output.log
touch output.log
cargo run &
tail -f output.log &