### build w docker

navigate to top level (sphinx-key)

`docker build -f broker/Dockerfile -t sphinx-key-broker .`

### test test locally

To run the broker test against the esp32-c3:

`cargo run -- --test`
