check_exists() {
    command -v "$1" > /dev/null
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
if ! check_exists espflash
then
    echo "espflash not installed!"
    echo "install with this command: cargo install espflash"
    exit 1
fi
cargo espflash erase-flash
cd factory &&
cargo espflash flash --release &&
cargo espflash save-image --release --chip esp32c3 factory.bin &&
espsecure.py sign_data factory.bin --version 2 --keyfile ../secure_boot_signing_key.pem &&
espflash write-bin 0x10000 factory.bin &&
cd ../sphinx-key &&
cargo espflash save-image --bin sphinx-key --release --chip esp32c3 sphinx-key.bin &&
espsecure.py sign_data sphinx-key.bin --version 2 --keyfile ../secure_boot_signing_key.pem &&
espflash write-bin 0x50000 sphinx-key.bin &&
cargo espflash monitor
