# sphinx-key

These notes were tested for macOS

### deps

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

`espflash /dev/tty.SLAB_USBtoUART target/riscv32imc-esp-espidf/debug/sphinx-key`

If the above command does not work, try this one below:

`espflash target/riscv32imc-esp-espidf/debug/sphinx-key`
