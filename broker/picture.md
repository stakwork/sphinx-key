```
     ┌───────┐
     │       │                  ┌───────────────────────┐
     │ HELLO │                  │                       │
     │       │                  │   VLS, LSS, CONTROL   │
     │       │                  │                       │
     └───┬───┘                  └─────┬─────────────────┘
         │                            │            ▲
         │                            │            │
         │                            │            │
         │                            │            │
internal_status                       │            │
         │                         msg_tx        link_tx
         │                            │            │
         │                            │            │
         ▼                            │            │
    ┌──────────┐                      ▼            │
    │          │                    ┌──────────────┴──┐     oneshot           ┌─────────────────┐
    │   SUBS   │                    │                 ├─────────────────────► │                 │
    │          │                    │  PUB AND WAIT   │     mqtt_tx           │  ROCKET         │
    └────┬─────┘                    │                 │ ◄─────────────────────┤                 │
         │                          └──┬──────────────┘                       │  /CONTROL POST  │
         │                             │           ▲                          │                 │
  status_sender                        │           │                          └─────────────────┘
         │                          oneshot        │
         │                             │         mqtt_tx
         ▼                             │           │
   ┌─────────────┐                     ▼           │
   │             │                 ┌───────────────┴───┐     lss_tx           ┌──────────────┐
   │ CLIENT LIST │                 │                   ├────────────────────► │              │
   │             │                 │    SIGNER LOOP    │                      │  LSS BROKER  │
   └───┬─────────┘                 │                   │     oneshot          │              │
       │     ▲                     │                   │ ◄────────────────────┤              │
       │     │                     └───────┬───────────┘                      └──────────────┘
   conn│     │dance_complete               │      ▲                                 ▲
       │     │                          write_vec │                                 │
       │     │                             │      │                              put_muts
       ▼     │                             │   read_raw                             │
  ┌──────────┴─────┐                       ▼      │                                 ▼
  │                │                     ┌────────┴┐                             ┌────────┐
  │ CONNECT DANCE  │                     │         │                             │        │
  │                │                     │   CLN   │                             │  LSS   │
  └───┬────────────┘                     │         │                             │        │
      │          ▲                       └─────────┘                             └────────┘
    init_tx      │
      │        oneshot
      ▼          │
  ┌──────────────┴──┐
  │                 │
  │ PUB AND WAIT    │
  │                 │
  └────┬────────────┘
       │       ▲
       │       │
    link_tx    │
       │       │
       │     init_tx
       ▼       │
      ┌────────┴┐
      │         │
      │  INIT   │
      │         │
      └─────────┘
```
