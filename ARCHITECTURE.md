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

- `VLS PROXY CLIENT`: Reads and writes `vls_protocol` messages via UNIX file descriptors. Lives in `broker/unix_fd.rs`.
- `SIGNER LOOP`: Loops on `vls_protocol` messages received on `VLS PROXY CLIENT`, sends them to `MQTT BROKER` via rust `std::sync::mpsc` thread communication channels. Lives in `broker/unix_fd.rs`.
- `MQTT BROKER`: Receives messages from `SIGNER LOOP` via `mpsc` channels, and sends them to `SPHINX KEY` over the internet via authenticated MQTT. Lives in `broker/mqtt.rs`.
- `MQTT CLIENT`: Receives MQTT messages from `MQTT BROKER` over the internet and sends them to `VLS PROTOCOL SIGNER` via `mpsc`.

Paste the code block above into asciiblock.com to make edits. Then press the download button on the top right of the pane to bring it back here.
