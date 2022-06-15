#### MULTIPLE DROMS: June 15th 2022

Over the past few days, we've been working on this hard blocker below:

```
Flashing has completed!
Commands:
    CTRL+R    Reset chip
    CTRL+C    Exit

�
 I�ESP-ROM:esp32c3-api1-20210207
Build:Feb  7 2021
rst:0x1 (POWERON),boot:0xc (SPI_FAST_FLASH_BOOT)
SPIWP:0xee
mode:DIO, clock div:1
load:0x3fcd6100,len:0x172c
load:0x403ce000 [_iram_data_end:??:??],len:0x928
load:0x403d0000 [_iram_data_end:??:??],len:0x2ce0
entry 0x403ce000 [_iram_data_end:??:??]
I (30) boot: ESP-IDF v4.4-dev-2825-gb63ec47238 2nd stage bootloader
I (30) boot: compile time 12:10:40
I (30) boot: chip revision: 3
I (33) boot_comm: chip revision: 3, min. bootloader chip revision: 0
I (41) boot.esp32c3: SPI Speed      : 80MHz
I (45) boot.esp32c3: SPI Mode       : DIO
I (50) boot.esp32c3: SPI Flash Size : 4MB
I (55) boot: Enabling RNG early entropy source...
I (60) boot: Partition Table:
I (64) boot: ## Label            Usage          Type ST Offset   Length
I (71) boot:  0 nvs              WiFi data        01 02 00009000 00006000
I (78) boot:  1 phy_init         RF data          01 01 0000f000 00001000
I (86) boot:  2 factory          factory app      00 00 00010000 003f0000
I (93) boot: End of partition table
I (98) boot_comm: chip revision: 3, min. application chip revision: 0
I (105) esp_image: segment 0: paddr=00010020 vaddr=3c060020 size=21a84h (137860) map
I (134) esp_image: segment 1: paddr=00031aac vaddr=3fc89c00 size=015ech (  5612) load
I (135) esp_image: segment 2: paddr=000330a0 vaddr=40380000 size=09a50h ( 39504) load
I (147) esp_image: segment 3: paddr=0003caf8 vaddr=50000010 size=00010h (    16) load
I (148) esp_image: segment 4: paddr=0003cb10 vaddr=00000000 size=04f90h ( 20368)
I (159) esp_image: segment 5: paddr=00041aa8 vaddr=3c081aa8 size=00150h (   336) map
I (165) esp_image: segment 6: paddr=00041c00 vaddr=00000000 size=0e418h ( 58392)
I (182) esp_image: segment 7: paddr=00050020 vaddr=42000020 size=52964h (338276) map
I (235) boot: Loaded app from partition at offset 0x10000
I (236) boot: Disabling RNG early entropy source...
E (236) boot: Image contains multiple DROM segments. Only the last one will be mapped.
```

Before reaching this point, we had successfully passed Core Lightning's `test_pay`[^1] test after replacing the native HSMD daemon with a software simulation of a remote hsmd communicating with the node via MQTT.[^2] Now, we were attempting to pass this test while running the signer logic from the Validating Lightning Signer project[^3] on our actual ESP32-C3,[^4] and everytime we flashed the software, the boot processed blocked with the message `Image contains multiple DROM segments. Only the last one will be mapped`, as shown above.

First, we performed a sanity check on the binary loaded on the chip, seeing for example if these multiple DROM segments were replicas of each other. We eventually didn't find any cause for concern there, so we moved higher up the stack in search for the root problem.

Next, we put together the smallest piece of self-standing code possible that would reproduce this error. This allowed us to narrow down exactly which lines of code in our codebase were causing this problem. It also made sure that this wasn't the result of loading an oversize binary onto our ESP32-C3, which had been a working hypthesis for the cause of the problem up until then.

After a lot of digging, we landed here:

```
use esp_idf_sys as _;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

fn main() {

    esp_idf_sys::link_patches();
    let secp = Secp256k1::new();

    let secret_key = SecretKey::from_slice(&[0xcd; 32]).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    
    let message = Message::from_slice(&[0xab; 32]).expect("32 bytes");

    let sig = secp.sign(&message, &secret_key);
    assert!(secp.verify(&message, &sig, &public_key).is_ok());

    println!("signature verified!");
    println!("Hello, world!");
}
```
Now the goal was to run this small piece of code on the ESP32-C3. If we could, then this would very likely mean that we had solved the problem.

[^1]: Test found in `lightning/tests/test_pay.py`
[^2]: See architecture diagram [here](ARCHITECTURE.md)
[^3]: See project homepage [here](https://gitlab.com/lightning-signer)
[^4]: The chip itself was mounted on the ESP32-C3 Mini Development Board found [here](https://www.sparkfun.com/products/18036)
