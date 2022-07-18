## Demo notes for signer against aws regtest cln node

### ESP32

- Log in to the ubuntu machine, connect the ESP, launch terminal, and `cd code/sphinx-key/sphinx-key`.
- Then launch `clearstorage`, and do a `ctrl-c` when you see the message `NVS cleared!` in the logs.
- Launch `buildkey`. This builds the signer software that will run on the esp.
- `flashkey`. This writes the signer software to the esp.
- Then do `mon`. Short for monitor, this restarts the ESP, and outputs its logs to the screen.
- Wait for the message `Waiting for data from the phone!`. The LED should blink green.
- On your phone connect to the Wifi `sphinxkey`. This is served from the ESP32.
- Launch the signer setup flow on the sphinx app, and input the following settings:

ESP IP address: `192.168.71.1`\
Broker IP address and port: `44.198.193.18:1883`\
SSID: ssid of a local wifi with access to the internet\
Password: password of the wifi from the previous step

- Once the setup is complete, the message `CONFIG SAVED` should appear. Check in the log right above that all the settings are correct.
- Press the `RST` button, to the right of the USB cable on the ESP. This does a hard reset of the ESP, and now launches the signer.
- The LED will first blink yellow while it is connecting to the wifi.
- When the signer is pinging for the broker, the LED on the ESP blinks purple.
- On the logs, you should see `BROKER IP AND PORT` and `LED STATUS: ConnectingToMqtt`

### CLN AWS

- You'll need 4 windows.
- SSH all of the windows onto the AWS EC2 instance
- Run `cleanup` in window A. This completely resets the regtest environment on the AWS instance.
- Then run `regd` in window A. This launches the regtest node.
- In window B, launch `aliced`. This launches alice, a generic regtest CLN node.
- In window C, run `alice-cli newaddr`.
- In the same window, run `touchwallet && genbtc {address of previous step} && blkdump`
- export BITCOIND_RPC_URL=http://localhost
- In window D, launch `bobd`. This launches bob, our MQTT remote signer node.
- On the ESP32, the LED should blink white when the signer is ready to sign for the node.
- Once its pubkey is logged, copy it.
- Back in window C, run `alice-cli connect {bob pubkey} localhost:20000`.
- Then `alice-cli fundchannel {bob pubkey} 100000`. This opens a 100'000 sat channel from alice to bob.
- Then do `blkdump` to generate a bunch of blocks and confirm the channel.

Keysend: `alice-cli keysend {bob pubkey} 1000` ( keysend of 1 satoshi from alice to bob ).\
Generate invoice: `bob-cli invoice 1000 {label} {description}`\
Pay invoice: `alice-cli pay {invoice}`\
Close channel: `alice-cli close {bob pubkey}`\
Get the node pubkey: `alice-cli getinfo`\
List the utxos of the node: `alice-cli listfunds`\
List the peers and channels of the node: `alice-cli listpeers`\
Stop the node: `alice-cli stop`\
Completely reset the regtest environment: `cleanup`
