```
                                   POWER UP                     ────── SOLID RED
                                        │
                                        ▼
                          YES  ┌───────────────────┐
                         ┌─────┤NVS GOT WIFI, KEYS?│
                         │     └────────┬──────────┘
                         │              │NO
                         │              │
                         │              ▼
    ┌─► RESET ───────────┼───────► SERVING WIFI AP              ────── YELLOW BLINK
    │   WIFI             │              │
    │                    │              │
    │                    │              │
    │                    │              ▼
    │                    │         CONNECTION RECEIVED          ────── YELLOW SOLID
    │                    │              │
    │                    │              │
    │                    │              │
    │                    │              ▼
    │                    └───────► GOT THE CREDENTIALS, KEYS    ────── ORANGE BLINK
    │                              ATTEMPTING TO CONNECT TO WIFI
    │                                   │
    │BUTTON PRESS                       │
    │                                   │
    │                                   ▼
┌───┴────────────────┬────►        WIFI CONNECTED,
│WIFI CONNECTIOW DOWN│             CONNECTING TO MQTT BROKER    ────── VIOLET BLINK
│        ==          │                  │
│  SLOW BLINK RED    │                  │
└────────────────────┘                  │
                                        ▼
┌─────────────────────┬───►   ┌──► CONNECTED TO MQTT BROKER,    ────── BLUE BLINKS
│ MQTT CONNECTION DOWN│       │    LISTENING TO MESSAGES               EVERY FIVE SECONDS
│        ==           │       │         │
│   FAST BLINK RED    │       │         │
└─────────────────────┘       │         │
                              │         ▼
                              └─── SIGNED A MESSAGE             ────── GREEN BLINK
```

Paste the code block above into `asciiblock.com` to make edits. Then press the download button on the top right of the pane to bring it back here.
