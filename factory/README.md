# Sphinx Key Factory App

The main function of this app is to write any `update.bin` files from the sd card to the flash of the ESP, and configure the ESP so that on the next boot, it boots the freshly written app.

## Background Reading

- Partition Tables: https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-guides/partition-tables.html
- Over-the-Air Updates: https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-reference/system/ota.html

## Flashing factory and sphinx-key

- First flash the factory app here using the usual `espflash` command, but add the `--partition-table` flag and point it to `table.csv` here. See `espflash -h` for more info.
- Then use `esptool.py` to flash the sphinx-key binary at offset `0xc0000`.
- Finally use this command to tell the ESP to boot the sphinx-key binary first ( there is no update to write to the ESP yet, so we don't boot the factory app ): `otatool.py switch_ota_partition --slot 0`
