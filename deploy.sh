MODE=debug
# MODE=release
check_exists() {
    command -v "$1" > /dev/null
}
check_port() {
    cargo espflash board-info "$1" &> /dev/null
}
if ! check_exists esptool.py
then
    echo "esptool.py not installed!"
    echo "install with this command: pip install esptool"
    exit 1
fi
if ! check_exists ldproxy
then
    echo "ldproxy not installed!"
    echo "install with this command: cargo install ldproxy"
    exit 1
fi
if ! check_exists cargo-espflash
then
    echo "cargo-espflash not installed!"
    echo "install with this command: cargo install cargo-espflash"
    exit 1
fi
if [ -z "$SSID" ]
then
    echo "Please set environment variable SSID to the SSID of the wifi you'll use to configure your sphinx-key."
    exit 1
fi
if [ -z "$PASS" ]
then
    echo "Please set environment variable PASS to the password of the wifi you'll use to configure your sphinx-key."
    exit 1
fi
if [ ${#PASS} -lt 8 ]
then
    echo "Please set PASS to a password longer than 7 characters."
    exit 1
fi
for FILE in /dev/tty.*
do
    # Check for port on macOS
    if check_port $FILE 
    then
        PORT=$FILE
        break
    fi
done
if [ -z "$PORT" ]
then
    # Check for port on linux
    if check_port /dev/ttyUSB0
    then
        PORT=/dev/ttyUSB0
    fi
fi
if [ -z "$PORT" ]
then
    echo "ESP likely not connected! Exiting now."
    echo "Make sure the ESP is connected with a data USB cable, and try again."
    exit 1
fi
esptool.py erase_flash &&
git pull &&
cd factory &&
cargo espflash --release $PORT &&
cd ../sphinx-key &&

if [ $MODE = "release" ]
then
    cargo build --release
else
    cargo build
fi &&

esptool.py --chip esp32-c3 elf2image target/riscv32imc-esp-espidf/$MODE/sphinx-key &&
esptool.py --chip esp32c3 -b 460800 --before=default_reset --after=hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x80000 target/riscv32imc-esp-espidf/$MODE/sphinx-key.bin &&
cargo espflash serial-monitor $PORT
