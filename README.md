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