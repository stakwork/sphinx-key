# sphinx-key

These notes were tested for macOS

### deps

`cd sphinx-key`

##### cargo nightly:

`rustup install nightly`

`rustup component add rust-src --toolchain nightly`

`rustup default nightly`

##### python 3.7 or higher is required

##### cargo sub-commands

`cargo install cargo-generate`

`cargo install ldproxy`

`cargo install espflash`

`cargo install espmonitor`

### build

`cargo build`

### flash

`espflash target/riscv32imc-esp-espidf/debug/sphinx-key`

### monitor

```sh
ls /dev/tty.*
ls /dev/cu.*
espmonitor /dev/tty.usbserial-1420
```

### clear NVS

espflash target/riscv32imc-esp-espidf/debug/clear
espmonitor /dev/tty.usbserial-1420

### cargo generate esp-rs

`cargo generate --git https://github.com/esp-rs/esp-idf-template cargo`

```
std support: true
v4.4
esp32c3
nightly
```

`cargo build`

### build with CC option

In this new esp-rs repo, find the path to your `riscv32-esp-elf-gcc` binary within the `.embuild` dir:

`export CC=/Users/evanfeenstra/code/sphinx-key/sphinx-key/sphinx-key/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-patch3-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc`

### get std features of vls:

Download a local copy of the `validating-lightning-signer` repo in the parent directory of this repo.

`git clone https://gitlab.com/lightning-signer/validating-lightning-signer.git`

in validating-lightning-signer/vls-protocol-signer/Cargo.toml `[features]`

add: `vls-std = ["vls-protocol/std"]`

### build sphinx-key

then in the sphinx-key dir, with the CC variable set as above:

`cargo build`

and flash using the instructions further above

### to tell sphinx-key where to find the MQTT broker:

clear the NVS with instructions above if sphinx-key has stale Wifi creds.\
restart sphinx key, then from computer connect to sphinxkey AP.\
go to `http://192.168.71.1/?broker=52.91.253.115%3A1883`.\
input internet wifi SSID and password, and the IP address of the broker.\
after pressing the ok button, restart the sphinx key, and wait for a MQTT connection.
