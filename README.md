# sphinx-key

### deps

##### cargo nightly:

`rustup install nightly`

`rustup component add rust-src --toolchain nightly`

##### python 3.7 or higher is required

##### cargo sub-commands

`cargo install cargo-generate`

`cargo install ldproxy`

`cargo install espflash`

`cargo install espmonitor`

### build

`cargo build`

### flash

`espflash /dev/ttyUSB0 target/riscv32imc-esp-espidf/debug/sphinx-key`

replace dev/ttyUSB0 with the usb where board is connected
