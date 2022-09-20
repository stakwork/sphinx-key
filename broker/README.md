### build w docker

navigate to top level (sphinx-key)

`docker build -f broker/Dockerfile -t sphinx-key-broker .`

### test locally

To run the broker test against the esp32-c3:

`cargo run -- --test`

### w docker

cid=$(docker create sphinx-key-broker)
docker cp $cid:/usr/src/sphinx-key-broker - > local-key-broker
docker rm -v $cid
