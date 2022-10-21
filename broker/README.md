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

### c-lightning

##### get the version 

`git describe --tags --long --always --match='v*.*'`

and only take the last 8 chars of the last string

or 

`docker run -it --entrypoint "/bin/bash" sphinx-cln`

`lightningd --version`

##### build c-lightning

docker build . -t sphinx-cln

docker tag sphinx-cln sphinxlightning/sphinx-cln-vls:0.1.4

docker push sphinxlightning/sphinx-cln-vls:0.1.4

##### testing

cargo run --bin sphinx-key-tester -- --log