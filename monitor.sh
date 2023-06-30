check_exists() {
    command -v "$1" > /dev/null
}
check_port() {
    cargo espflash board-info --port "$1" &> /dev/null
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
cargo espflash monitor --port $PORT
