# sphinx-key

These notes were tested for macOS

### set CFLAGS

Before building the sphinx-key esp software, run this:

`export CFLAGS=-fno-pic`

### find your esp GCC 

Find the path to your `riscv32-esp-elf-gcc` binary within the `.embuild` dir:

`export CC=$PWD/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-patch3-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc`

### build test

`cargo build --features pingpong`

### flash test

`espflash target/riscv32imc-esp-espidf/debug/sphinx-key --monitor`

### build release

`cargo build --release`

### flash release

`esptool.py --chip esp32c3 elf2image target/riscv32imc-esp-espidf/release/sphinx-key`

`esptool.py --chip esp32c3 -p /dev/tty.usbserial-1420 -b 460800 --before=default_reset --after=hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x10000 target/riscv32imc-esp-espidf/release/sphinx-key.bin`

### monitor

```sh
ls /dev/tty.*
ls /dev/cu.*
espmonitor /dev/tty.usbserial-1420
```

### configure the hardware

make a seed: `./sphinx-key/newseed.sh` 

make a `.env` file like:

```
SSID=my_ssid
PASS=my_wifi_password
BROKER=my_ip:1883
SEED=my_seed
NETWORK=regtest
```

connect to the `sphinxkey` network on your computer

`cargo run --bin config`

This will encrypt your seed and send to the hardware, along with your home wifi information and broker address

# dependencies

`cd sphinx-key`

##### cargo nightly:

`rustup install nightly`

`rustup component add rust-src --toolchain nightly`

`rustup default nightly`

##### python 3.7 or higher is required

##### cargo sub-commands:

`cargo install cargo-generate`

`cargo install ldproxy`

`cargo install espflash`

`cargo install espmonitor`

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

### to tell sphinx-key where to find the MQTT broker:

clear the NVS with instructions above if sphinx-key has stale Wifi creds.\
restart sphinx key, then from computer connect to sphinxkey AP.\
go to `http://192.168.71.1/?broker=52.91.253.115%3A1883`.\
input internet wifi SSID and password, and the IP address of the broker.\
after pressing the ok button, restart the sphinx key, and wait for a MQTT connection.

### espflash notes

espflash save-image esp32-c3 target/riscv32imc-esp-espidf/release/sphinx-key ./test-flash

espflash board-info

export CC=$PWD/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc


cargo +nightly build --release

esptool.py --chip esp32c3 elf2image target/riscv32imc-esp-espidf/release/sphinx-key

esptool.py --chip esp32c3 -p /dev/tty.usbserial-1420 -b 460800 --before=default_reset --after=hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x10000 target/riscv32imc-esp-espidf/release/sphinx-key.bin

espmonitor /dev/tty.usbserial-1420

### config

./sphinx-key/rando.sh

make your .env

cargo run --bin config
