check_port() {
    cargo espflash board-info "$1" &> /dev/null
}
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
