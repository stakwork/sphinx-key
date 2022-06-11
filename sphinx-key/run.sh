cargo build --features pingpong
espflash target/riscv32imc-esp-espidf/debug/sphinx-key
espmonitor /dev/tty.usbserial-1420