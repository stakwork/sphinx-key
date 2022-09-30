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
for FILE in /dev/tty.*
do
    if check_port $FILE 
    then
        PORT=$FILE
        break
    fi
done
if [ -z "$PORT" ]
then
    echo "ESP likely not connected! Exiting now."
    echo "Make sure the ESP is connected with a data USB cable, and try again."
    exit 1
fi
git pull
cd factory
cargo espflash --release $PORT
cd ../sphinx-key
cargo build
esptool.py --chip esp32-c3 elf2image target/riscv32imc-esp-espidf/debug/sphinx-key
esptool.py --chip esp32c3 -b 460800 --before=default_reset --after=hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x80000 target/riscv32imc-esp-espidf/debug/sphinx-key.bin
cargo espflash serial-monitor $PORT
