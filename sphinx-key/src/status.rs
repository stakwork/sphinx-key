#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
pub enum Status {
    Waiting,
    Starting,
    MountingSDCard,
    SyncingTime,
    WifiAccessPoint,
    Configuring,
    ConnectingToWifi,
    ConnectingToMqtt,
    Connected,
    Signing,
    Ota,
    Reset1a,
    Reset1,
    Reset2a,
    Reset2,
    Reset3,
}
