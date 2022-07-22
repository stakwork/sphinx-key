## Instructions to build and run the Sphinx Key signer

### Hardware setup

#### Picture

![Spi connections picture](docs/spi_connections.jpeg)

##### Sparkfun Shopping List

- ESP32-C3 Mini Development Board: https://www.sparkfun.com/products/18036
- SparkFun microSD Transflash Breakout: https://www.sparkfun.com/products/544
- Breadboard - Self-Adhesive (White): https://www.sparkfun.com/products/12002
- Break Away Headers - Straight: https://www.sparkfun.com/products/116
- Jumper Wire Kit - 140pcs: https://www.sparkfun.com/products/124

##### Soldering and SD Card Format
- You'll also need a sizeable SD Card formatted using the FAT32 filesystem.
- Once you have the parts, solder the breakaway headers to the microSD card board as shown in the picture above.

Now follow the table below and the picture above to make all the connections:

SD card pin | ESP32-C3-DevKitM-1 v1.0 | Notes
------------|-------------------------|--------------------
 DO         | GPIO6                   | Pin numbered 6 on board, same for the others below
 CS         | GPIO1                   | 
 SCK        | GPIO5                   |
 DI         | GPIO4                   |
 VCC        | 3V3                     |
 GND        | GND                     |

### Software setup ( MacOS )

- Make sure you have the Apple Command Line Developer tools installed on your machine. If not, run `xcode-select --install`
- Install rust. You can grab the installation command at https://www.rust-lang.org/tools/install
- Install brew. Get the installation command at https://brew.sh/
- Install python3 and virtualenv. You can run `brew install python3 virtualenv` if necessary.
- Run the following commands:
```
rustup install nightly
rustup component add rust-src --toolchain nightly
cargo install cargo-generate ldproxy espflash espmonitor
```

### Hello World

Before we build and run the signer, we'll walk through a generic hello world to make sure the environment is all working:

- Type `cd ~`. This places you in your home directory.
- Run the command below, and set the following settings when prompted: `Project Name=tiny-esp32, MCU=esp32c3, ESP-IDF native build version=v4.4, STD support=true, Configure project to use Dev Containers=false`
```
cargo generate --vcs none --git https://github.com/esp-rs/esp-idf-template cargo
```
- `cd tiny-esp32`
- `cargo build`
- Once the build is complete, run `ls /dev/tty.*` and note the files you see in that directory.
- Plug in the ESP32-C3 dev board to your computer via Micro-USB, and again run `ls /dev/tty.*`. A new file should now appear, similar to this one `/dev/tty.usbserial-1420`
- Run `export FLASHPORT=[full file path noted in the previous step]`. In my case: `export FLASHPORT=/dev/tty.usbserial-1420`
- Now with the ESP32 dev board still plugged in, run:
```
espflash --monitor $FLASHPORT target/riscv32imc-esp-espidf/debug/tiny-esp32
```
- This flashes the program onto the dev board, and then monitors the logs as soon as the program starts to run. By the end of execution, you should see a little `Hello World` log on your screen.
- You are now ready to build, flash, and run the signer :)

### Signer

- `cd ~`
- `git clone https://github.com/stakwork/sphinx-key.git`
- `cd sphinx-key/sphinx-key`
- `virtualenv venv`
- `source venv/bin/activate`
- `pip3 install --upgrade pip`
- `pip3 install esptool`
- `export CFLAGS=-fno-pic`
- `export CC=$HOME/tiny-esp32/.embuild/espressif/tools/riscv32-esp-elf/*/riscv32-esp-elf/bin/riscv32-esp-elf-gcc`
- `cargo build`. You are now building the sphinx-key signer!
- `esptool.py --chip esp32-c3 elf2image target/riscv32imc-esp-espidf/debug/sphinx-key`

Now flash the software onto the dev board using this command:
```
esptool.py --chip esp32c3 -p $FLASHPORT -b 460800 --before=default_reset --after=hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x10000 target/riscv32imc-esp-espidf/debug/sphinx-key.bin
```
And then print the logs on your screen with this command:
```
espmonitor $FLASHPORT
```
