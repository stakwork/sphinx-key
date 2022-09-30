# Sphinx Key Factory App

The main function of this app is to write any `update.bin` files from the sd card to the flash of the ESP, and configure the ESP so that on the next boot, it boots the freshly written app.

## Background Reading

- Partition Tables: https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-guides/partition-tables.html
- Over-the-Air Updates: https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-reference/system/ota.html
