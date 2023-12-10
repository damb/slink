use std::fmt;

/// Command to request information about the SeedLink server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Info {
    item: InfoItem,
}

impl Info {
    pub const NAME: &'static str = "info";

    pub fn new(item: InfoItem) -> Self {
        Self { item }
    }
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", Info::NAME, self.item)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InfoItem {
    Id,
    Capabilities,
    Stations,
    Streams,
    Gaps,
    Connections,
    All,
}

impl fmt::Display for InfoItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = match self {
            InfoItem::Id => "id",
            InfoItem::Capabilities => "capabilities",
            InfoItem::Stations => "stations",
            InfoItem::Streams => "streams",
            InfoItem::Gaps => "gaps",
            InfoItem::Connections => "connections",
            InfoItem::All => "all",
        };
        write!(f, "{}", item)
    }
}
