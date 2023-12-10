use std::fmt;

/// Command to request station data during handshaking.
///
/// Note that this command enables *multi-station* mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Station {
    network: Option<String>,
    station: String,
}

impl Station {
    pub const NAME: &'static str = "station";

    pub fn new(station: &str, network: Option<String>) -> Self {
        Self {
            network,
            station: station.to_string(),
        }
    }
}

impl fmt::Display for Station {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut net_str = String::new();
        if let Some(net) = &self.network {
            net_str = format!(" {}", net);
        }
        write!(f, "{} {}{}", Station::NAME, self.station, net_str)
    }
}
