```
                 ┌──────────────────┐
                 │     CORE LN      │
                 │                  │
                 │                  │
                 │   - lightningd   │
                 │   - channeld     │
                 │   - openingd     │
                 │                  │
                 └────────┬─────────┘
                          │
                          │
                          │ UNIX FILE DESCRIPTORS
                          │
                          │
               ┌──────────┼───────────┐
               │          │           │
               │ ┌────────▼─────────┐ │
               │ │    CLN CLIENT    │ │
               │ │                  │ │
               │ │                  │ │
BROKER         │ │   SIGNER LOOP    │ │   ---------   BITCOIND
               │ └────────┬─────────┘ │
               │          │           │
               │          │           │   
               │          | MPSC      │    
               │          │           │
               │          │           │
               │   ┌──────▼──────┐    │
               │   │ MQTT BROKER │    │
               │   └──────┬──────┘    │
               │          │           │
               └──────────┼───────────┘
                          │
                          │
                          │ MQTT
                          │
                          │
             ┌────────────┼────────────┐
             │            │            │
             │     ┌──────▼──────┐     │
             │     │ MQTT CLIENT │     │
             │     └──────┬──────┘     │
             │            │            │
SPHINX-KEY   │            │            │
             │            │ MPSC       │
             │            │            │
             │            │            │
             │ ┌──────────▼──────────┐ │
             │ │ VLS PROTOCOL SIGNER │ │
             │ └──────────┬──────────┘ │
             │            │            │
             │            │            │
             │            │ HAL SPI    │
             │            │            │
             │            │            │
             │    ┌───────▼───────┐    │
             │    │ SD CARD, LEDs │    │
             │    └───────────────┘    │
             │                         │
             └─────────────────────────┘
```

## Key Components

- `CORE LN`: Sends `vls_protocol` messages to `CLN CLIENT` via UNIX file descriptors.
- `CLN CLIENT`: Reads and writes `vls_protocol` messages via UNIX file descriptors. Lives in `broker/unix_fd.rs`.
- `SIGNER LOOP`: Loops on `vls_protocol` messages received on `CLN CLIENT`, sends them to `MQTT BROKER` via rust `std::sync::mpsc` thread communication channels. Lives in `broker/unix_fd.rs`.
- `MQTT BROKER`: Receives messages from `SIGNER LOOP` via `mpsc` channels, and sends them to `SPHINX KEY` over the internet via authenticated MQTT. Lives in `broker/mqtt.rs`.
- `BITCOIND`: Provides on-chain data to `BROKER` for validation of the operations of `VLS PROTOCOL SIGNER`.
- `MQTT CLIENT`: Receives MQTT messages from `MQTT BROKER` over the internet and sends them to `VLS PROTOCOL SIGNER` via `mpsc` channels. `MQTT CLIENT` lives in `sphinx-key/src/conn/mqtt.rs` and `VLS PROTOCOL SIGNER` lives in `signer/src/lib.rs`.
- `SD CARD`: Persists data from `VLS PROTOCOL SIGNER`. Communicates with `VLS PROTOCOL SIGNER` via the SPI protocol implemented in `esp_idf_hal::spi`.
- `LEDs`: Show users and engineers the state of the sphinx-key, for both UX and debugging. Also communicate with `VLS PROTOCOL SIGNER` via SPI.

Paste the code block above into `asciiblock.com` to make edits. Then press the download button on the top right of the pane to bring it back here.
