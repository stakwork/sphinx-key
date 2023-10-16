

#### test control messages

cargo run --bin sphinx-key-tester -- --test --log

cd broker 
cargo run -- --test

cargo run --bin ctrl

#### sample cmd.json file
```json
{
  "Ota": {
      "url": "http://192.168.86.21:12346/sphinx-update-",
      "version": 0
  }
}
```

#### sample .env file

```
SSID="foo"
PASS="bar"
BROKER="44.198.193.18:1883"
BROKER_URL="http://44.198.193.18:30000/api"
SEED=c7629e0f2edf1be66f01c0824022c5d30756ffa0f17213d2be463a458d200803
NONCE="0"
```

### sample update server
cd into the directory where you placed the .bin file you want to send to the signers
mini_httpd -p 12346
then use something like the cmd.json example above to tell signer about the update
