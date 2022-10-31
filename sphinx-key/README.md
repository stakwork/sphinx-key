## Instructions to build and run the Sphinx Key signer

### Software setup ( MacOS )

- Make sure you have the Apple Command Line Developer tools installed on your machine. If not, run `xcode-select --install`
- Install rust. You can grab the installation command at https://www.rust-lang.org/tools/install
- Install brew. Get the installation command at https://brew.sh/
- Install python3. You can run `brew install python3` if necessary.
- Run the following commands (the last one will take a while, go for a walk or something 😀):
```
pip install esptool
rustup install nightly
rustup component add rust-src --toolchain nightly
cargo install cargo-espflash ldproxy
```

### Signer

- Plug in the ESP32-C3 dev board to your computer via data-enabled micro-USB.
> **Note**
> The only use of the data usb connection to the signer is to write the program binary to flash memory - during operation, the usb connection is only used for power.

- `cd ~`
- `git clone https://github.com/stakwork/sphinx-key.git`
- `cd sphinx-key`
- `export SSID=wifi_name_you'll_use_to_configure_sphinx_key`
- `export PASS=password_of_wifi_you'll_use_to_configure_sphinx_key`
- `./deploy.sh`. This commands takes a while, it builds and flashes everything!
- You will eventually be shown the logs of the signer.
- Wait for the message `Waiting for data from the phone!`. The LED should blink green.
- Open a new terminal window, and `cd ~/sphinx-key/tester && cargo build`
- In the `~/sphinx-key/tester` directory, create a file `.env` with the settings shown below:

```
SSID="foo" # name of your home wifi - signer will use that to connect to the internet and ping the remote node
PASS="bar" # password of your home wifi
BROKER="00.00.00.00:0000" # IP address and port your broker is listening on on your remote server.
NETWORK="regtest"
SEED=c7629e0f2edf1be66f01c0824022c5d30756ffa0f17213d2be463a458d200803 # you can use the script at ~/sphinx-key/sphinx-key/newseed.sh to generate a fresh seed.
```

- Connect to the very first wifi network you specified above, enter the password, and then in `~/sphinx-key/tester` run `cargo run --bin config`.
> **Note**
> The sphinxkey network does not grant access to the internet, so ignore any warnings of that fact :)

- Once the setup is complete, the ESP will restart and attempt to connect to wifi.
- The LED will first blink yellow while it is connecting to the wifi.
- When the signer is pinging for the broker, the LED on the ESP blinks purple.
- On the logs, you should see `BROKER IP AND PORT` and `LED STATUS: ConnectingToMqtt`
- Now that the signer is configured, and pinging for the node, we'll proceed with setting up the node on the remote server.
- You can take a break here if you want, just unplug and plug the signer back in - all the settings you configured up until now are written to non-volatile flash memory.
- After you plug the signer back in, launch `cd ~/sphinx-key && ./monitor.sh` to print the logs to your screen once again.

### Remote Node Setup


### How to launch the signer again

- Plug in the ESP32-C3 dev board to your computer via data-enabled micro-USB.
- `cd ~/sphinx-key`
- `./deploy.sh`

### How to completely reset the signer

- Plug in your ESP to your computer.
- `esptool.py erase_flash`
- Next, unplug your ESP32 from your computer.
- Take out the SD card from its slot, and use your computer to clear all the data on it. Place it back in its slot after you've done so.
- You can now go to the section above to get going again.


### DIY Hardware Setup

#### Picture

![Spi connections picture](docs/spi_connections.jpeg)

##### Sparkfun Shopping List

- ESP32-C3 Mini Development Board: https://www.sparkfun.com/products/18036
- SparkFun microSD Transflash Breakout: https://www.sparkfun.com/products/544
- Breadboard - Self-Adhesive (White): https://www.sparkfun.com/products/12002
- Break Away Headers - Straight: https://www.sparkfun.com/products/116
- Jumper Wire Kit - 140pcs: https://www.sparkfun.com/products/124

##### Soldering and SD Card Format

> **Warning**
> This signer currently does not work with SD cards that come with the UHS-I feature.

- You'll need a microSD card formatted using the FAT32 filesystem.
  - On MacOS, go to `Disk Utility`.
  - Click on the SDCard's disk in the left hand pane.
  - Click on the `Erase` button on the cetner top toolbar.
  - Then choose `MS-DOS (FAT)` for the format.
  - Finally click on `Erase`, at the bottom right of the dialogue box.
- Also make sure you have a micro-USB cable capable of transferring data.
- Once you have the parts, solder the breakaway headers to the microSD card board as shown in the picture above.

Now follow the table below and the picture above to make all the connections:

SD card pin | ESP32-C3-DevKitM-1 v1.0 | Notes
------------|-------------------------|--------------------
 DO         | GPIO2                   | Pin numbered 6 on board, same for the others below
 CS         | GPIO10                  | No need for any of the 10kOhm resistors mentioned in docs as of July 2022
 SCK        | GPIO6                   |
 DI         | GPIO7                   |
 VCC        | 3V3                     |
 GND        | GND                     |

- Soon after, the LED should start to blink white, which means your signer is now connected to your node, and is ready for normal operation.
