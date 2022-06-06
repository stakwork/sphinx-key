                            POWER UP ───────────────────────────────────────── SOLID RED
                                 │
                                 │
                                 │
                                 ▼
    ┌──────────────────────►SERVING WIFI AP  ───────────────────────────────── YELLOW BLINK
    │                            │
    │                            │
    │                            │
    │                            ▼
    │                       CONNECTION RECEIVED ────────────────────────────── YELLOW SOLID
    │                            │
    │                            │
    │                            │
    │                            ▼
FAILED TO CONNECT           GOT THE CREDENTIALS, KEYS    ───────────────────── ORANGE BLINK
TO WIFI                     ATTEMPTING TO CONNECT TO WIFI
    ▲                            │
    └────────────────────────────┤
                                 │
                                 ▼
FAILED TO CONNECT MQT──────►CONNECTING TO MQTT BROKER ──────────────────────── VIOLET BLINK
    ▲                            │
    └────────────────────────────┤
                                 │
                                 ▼
                            CONNECTED TO MQTT BROKER, ──────────────────────── BLUE BLINK
                            LISTENING TO MESSAGES
                                 │
                                 │
                                 │
                                 ▼
                            SIGNED A MESSAGE ───────────────────────────────── GREEN BLINK
