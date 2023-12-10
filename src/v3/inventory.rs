use serde::{Deserialize, Deserializer};

use time::macros::format_description;
use time::{PrimitiveDateTime, OffsetDateTime};

// TODO(damb): 
//  - use u64 instead of i32 for sequence numbers
//  - validate with SeedLink v3

/// Structure representing a station in the inventory
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename(deserialize = "snake_case"))]
pub struct Station {
    /// Network code
    #[serde(rename = "@network")]
    pub network: String,
    /// Station code
    #[serde(rename = "@name")]
    pub code: String,
    /// Description
    #[serde(rename = "@description")]
    pub description: String,
    /// First packet sequence number
    #[serde(rename = "@begin_seq", deserialize_with = "deserialize_seq_num")]
    pub begin_seq: i32,
    /// Packet sequence number of the most recent packet
    #[serde(rename = "@end_seq", deserialize_with = "deserialize_seq_num")]
    pub end_seq: i32,

    /// Streams
    pub stream: Option<Vec<Stream>>,
}

/// Stream type enumeration
#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub enum StreamType {
    #[serde(rename = "D")]
    Data,
    #[serde(rename = "E")]
    Event,
    #[serde(rename = "C")]
    Calibration,
    #[serde(rename = "O")]
    Blockette,
    #[serde(rename = "T")]
    Timing,
    #[serde(rename = "L")]
    Log,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename(deserialize = "stream"))]
pub struct Stream {
    /// Location code
    #[serde(rename = "@location")]
    pub location: String,
    /// Channel code
    #[serde(rename = "@seedname")]
    pub channel: String,
    /// Stream type
    #[serde(rename = "@type")]
    pub stream_type: StreamType,

    /// Time of the first buffered packet
    #[serde(rename = "@begin_time", deserialize_with = "deserialize_datetime")]
    pub begin_time: OffsetDateTime,
    /// Time of the last buffered packet
    #[serde(rename = "@end_time", deserialize_with = "deserialize_datetime")]
    pub end_time: OffsetDateTime,
}

/// Struct representing the SeedLink server's stream information available.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename(deserialize = "seedlink"))]
pub struct Inventory {
    pub station: Vec<Station>,
}

fn deserialize_seq_num<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let buf = Deserialize::deserialize(deserializer)?;
    Ok(i32::from_str_radix(buf, 16).map_err(D::Error::custom)?)
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let buf = Deserialize::deserialize(deserializer)?;
    let format = format_description!(
        "[year][ignore count:1][month][ignore count:1][day] [hour]:[minute]:[second][optional [.[subsecond]]]"
    );
    Ok(PrimitiveDateTime::parse(buf, &format).map_err(D::Error::custom)?.assume_utc())
}

#[cfg(test)]
mod tests {

    use quick_xml::de::from_str;
    use time::macros::datetime;

    use super::{Inventory, Station, Stream, StreamType};

    #[test]
    #[should_panic]
    fn deserialize_empty_inventory() {
        let xml = r#"<?xml version="1.0"?>
            <seedlink software="HMB SeedLink v0.1 (2018.351)" organization="GEOFON" started="2021/03/30 08:50:25.0617">
            </seedlink>"#;
        let _: Inventory = from_str(xml).unwrap();
    }

    #[test]
    fn deserialize_station_no_streams() {
        let xml = r#"<?xml version="1.0"?>
            <seedlink software="HMB SeedLink v0.1 (2018.351)" organization="GEOFON" started="2021/03/30 08:50:25.0617">
            <station name="VNA1" network="AW" description="Station Neumayer OBS, Antarctica" begin_seq="563200" end_seq="582751" stream_check="enabled"/>
            </seedlink>"#;

        let inv: Inventory = from_str(xml).unwrap();
        let sta = Station {
            network: "AW".to_string(),
            code: "VNA1".to_string(),
            description: "Station Neumayer OBS, Antarctica".to_string(),
            begin_seq: 5648896,
            end_seq: 5777233,
            stream: None,
        };

        assert_eq!(inv, Inventory { station: vec![sta] });
    }

    #[test]
    fn deserialize_station_multi_streams() {
        let xml = r#"<?xml version="1.0"?>
            <seedlink software="HMB SeedLink v0.1 (2018.351)" organization="GEOFON" started="2021/03/30 08:50:25.0617">
            <station name="TRML" network="YU" description="TRML" begin_seq="3684001" end_seq="3684501" stream_check="enabled">
                <stream location="" seedname="HHZ" type="D" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:57.2700" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
                <stream location="" seedname="HHE" type="D" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:53.2200" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
                <stream location="" seedname="HHN" type="D" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:58.0100" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
            </station>
            </seedlink>"#;

        let hhz = Stream {
            location: "".to_string(),
            channel: "HHZ".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:57.2700 UTC),
        };
        let hhe = Stream {
            location: "".to_string(),
            channel: "HHE".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:53.2200 UTC),
        };
        let hhn = Stream {
            location: "".to_string(),
            channel: "HHN".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:58.0100 UTC),
        };
        let inv: Inventory = from_str(xml).unwrap();
        let sta = Station {
            network: "YU".to_string(),
            code: "TRML".to_string(),
            description: "TRML".to_string(),
            begin_seq: 57163777,
            end_seq: 57165057,
            stream: Some(vec![hhz, hhe, hhn]),
        };

        assert_eq!(inv, Inventory { station: vec![sta] });
    }

    #[test]
    fn deserialize_station_multi_streams_and_log() {
        let xml = r#"<?xml version="1.0"?>
            <seedlink software="HMB SeedLink v0.1 (2018.351)" organization="GEOFON" started="2021/03/30 08:50:25.0617">
            <station name="TRML" network="YU" description="TRML" begin_seq="3684001" end_seq="3684501" stream_check="enabled">
                <stream location="" seedname="HHZ" type="D" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:57.2700" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
                <stream location="" seedname="HHE" type="D" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:53.2200" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
                <stream location="" seedname="HHN" type="D" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:58.0100" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
                <stream location="" seedname="LOG" type="L" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:58.0120" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
            </station>
            </seedlink>"#;

        let hhz = Stream {
            location: "".to_string(),
            channel: "HHZ".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:57.2700 UTC),
        };
        let hhe = Stream {
            location: "".to_string(),
            channel: "HHE".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:53.2200 UTC),
        };
        let hhn = Stream {
            location: "".to_string(),
            channel: "HHN".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:58.0100 UTC),
        };
        let log = Stream {
            location: "".to_string(),
            channel: "LOG".to_string(),
            stream_type: StreamType::Log,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:58.0120 UTC),
        };
        let inv: Inventory = from_str(xml).unwrap();
        let sta = Station {
            network: "YU".to_string(),
            code: "TRML".to_string(),
            description: "TRML".to_string(),
            begin_seq: 57163777,
            end_seq: 57165057,
            stream: Some(vec![hhz, hhe, hhn, log]),
        };

        assert_eq!(inv, Inventory { station: vec![sta] });
    }

    #[test]
    fn deserialize_station_single_stream() {
        let xml = r#"<?xml version="1.0"?>
            <seedlink software="HMB SeedLink v0.1 (2018.351)" organization="GEOFON" started="2021/03/30 08:50:25.0617">
            <station name="TRML" network="YU" description="TRML" begin_seq="3684001" end_seq="3684501" stream_check="enabled">
                <stream location="" seedname="HHZ" type="D" begin_time="2012/12/29 14:18:45.8900" end_time="2012/12/29 14:37:57.2700" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
            </station>
            </seedlink>"#;

        let hhz = Stream {
            location: "".to_string(),
            channel: "HHZ".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:57.2700 UTC),
        };
        let inv: Inventory = from_str(xml).unwrap();
        let sta = Station {
            network: "YU".to_string(),
            code: "TRML".to_string(),
            description: "TRML".to_string(),
            begin_seq: 57163777,
            end_seq: 57165057,
            stream: Some(vec![hhz]),
        };

        assert_eq!(inv, Inventory { station: vec![sta] });
    }

    #[test]
    fn deserialize_station_single_stream_no_subseconds() {
        let xml = r#"<?xml version="1.0"?>
            <seedlink software="HMB SeedLink v0.1 (2018.351)" organization="GEOFON" started="2021/03/30 08:50:25.0617">
            <station name="TRML" network="YU" description="TRML" begin_seq="3684001" end_seq="3684501" stream_check="enabled">
                <stream location="" seedname="HHZ" type="D" begin_time="2012/12/29 14:18:45" end_time="2012/12/29 14:37:57" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
            </station>
            </seedlink>"#;

        let hhz = Stream {
            location: "".to_string(),
            channel: "HHZ".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45 UTC),
            end_time: datetime!(2012-12-29 14:37:57 UTC),
        };
        let inv: Inventory = from_str(xml).unwrap();
        let sta = Station {
            network: "YU".to_string(),
            code: "TRML".to_string(),
            description: "TRML".to_string(),
            begin_seq: 57163777,
            end_seq: 57165057,
            stream: Some(vec![hhz]),
        };

        assert_eq!(inv, Inventory { station: vec![sta] });
    }

    #[test]
    fn deserialize_station_single_stream_dashed_date() {
        let xml = r#"<?xml version="1.0"?>
            <seedlink software="HMB SeedLink v0.1 (2018.351)" organization="GEOFON" started="2021/03/30 08:50:25.0617">
            <station name="TRML" network="YU" description="TRML" begin_seq="3684001" end_seq="3684501" stream_check="enabled">
                <stream location="" seedname="HHZ" type="D" begin_time="2012-12-29 14:18:45.8900" end_time="2012-12-29 14:37:57.2700" begin_recno="0" end_recno="0" gap_check="disabled" gap_treshold="0"/>
            </station>
            </seedlink>"#;

        let hhz = Stream {
            location: "".to_string(),
            channel: "HHZ".to_string(),
            stream_type: StreamType::Data,
            begin_time: datetime!(2012-12-29 14:18:45.8900 UTC),
            end_time: datetime!(2012-12-29 14:37:57.2700 UTC),
        };
        let inv: Inventory = from_str(xml).unwrap();
        let sta = Station {
            network: "YU".to_string(),
            code: "TRML".to_string(),
            description: "TRML".to_string(),
            begin_seq: 57163777,
            end_seq: 57165057,
            stream: Some(vec![hhz]),
        };

        assert_eq!(inv, Inventory { station: vec![sta] });
    }
}
