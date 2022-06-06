```
                               POWER UP ────────────────────────── SOLID RED
                                    │
                                    ▼
                      YES  ┌───────────────────┐
                     ┌─────┤NVS GOT WIFI, KEYS?│
                     │     └────────┬──────────┘
                     │              │NO
                     │              │
                     │              ▼
    ┌─► RESET ─────► │         SERVING WIFI AP  ────────────────── YELLOW BLINK
    │   WIFI         │              │
    │                │              │
    │                │              │
    │                │              ▼
    │                │         CONNECTION RECEIVED ─────────────── YELLOW SOLID
    │                │              │
    │                │              │
    │                │              │
    │                │              ▼
    │                └───────► GOT THE CREDENTIALS, KEYS    ────── ORANGE BLINK
    │                          ATTEMPTING TO CONNECT TO WIFI
    │                               │
    │BUTTON PRESS                   │
    │                               │
    │                               ▼
┌───┴────────────────┬────►    WIFI CONNECTED,
│WIFI CONNECTIOW DOWN│         CONNECTING TO MQTT BROKER ───────── VIOLET BLINK
│        ==          │              │
│  SLOW BLINK RED    │              │
└────────────────────┘              │
                                    ▼
┌─────────────────────┬───►    CONNECTED TO MQTT BROKER, ───────── BLUE BLINK
│ MQTT CONNECTION DOWN│        LISTENING TO MESSAGES
│        ==           │             │
│   FAST BLINK RED    │             │
└─────────────────────┘             │
                                    ▼
                               SIGNED A MESSAGE ────────────────── GREEN BLINK
```
