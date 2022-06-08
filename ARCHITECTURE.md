```
  ┌──────────────────┐
  │     CORE LN      │
  │                  │
  │                  │
  │ VLS PROXY SERVER │
  └────────┬─────────┘
           │
           │UNIX FILE DESCRIPTORS
           │
           │
  ┌────────▼─────────┐
  │ VLS PROXY CLIENT │
  │                  │
  │                  │
  │   SIGNER LOOP    │
  └────────┬─────────┘
           │
           │MPSC
           │
           │
    ┌──────▼──────┐
    │ MQTT BROKER │
    └──────┬──────┘
           │
           │MQTT
           │
           │
    ┌──────▼──────┐
    │ MQTT CLIENT │
    └──────┬──────┘
           │
           │MPSC
           │
           │
┌──────────▼──────────┐
│ VLS PROTOCOL SIGNER │
└─────────────────────┘
```

## Key Components

- `VLS PROXY CLIENT`: Reads and writes `vls_protocol` messages via UNIX file descriptors. Lives in `broker/unix_fd.rs`.
- `SIGNER LOOP`: Loops on `vls_protocol` messages received on the `VLS PROXY CLIENT`, sends them to `MQTT BROKER` via rust `std::sync::mpsc` rust thread communication channels. Lives in `broker/unix_fd.rs`.
- `MQTT BROKER`: Receives messages from `SIGNER LOOP` via `mpsc` channels, and sends them to `SPHINX KEY` over the internet via authenticated MQTT. Lives in `broker/mqtt.rs`.
- `MQTT CLIENT`: Receives MQTT messages from `MQTT BROKER` over the internet and sends them to `VLS PROTOCOL SIGNER` via `mpsc`.
