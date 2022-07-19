CFLAGS=-fno-pic

CC=$PWD/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-patch3-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc

cargo +nightly build --release

esptool.py --chip esp32c3 elf2image target/riscv32imc-esp-espidf/release/sphinx-key

esptool.py --chip esp32c3 -p /dev/tty.usbserial-1420 -b 460800 --before=default_reset --after=hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x10000 target/riscv32imc-esp-espidf/release/sphinx-key.bin

espmonitor /dev/tty.usbserial-1420