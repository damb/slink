// use std::fs::File;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::time::Duration;

use futures::TryStreamExt;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use tokio::fs::OpenOptions;
use tokio::io::{self, AsyncWrite, AsyncWriteExt};
use tracing::{info, warn};
use tracing_subscriber;

use clap::{Parser, ValueEnum};

use mseed::MSControlFlags;
use slink::DEFAULT_PORT;
use slink::{Client, DataTransferMode, FDSNSourceId, SeedLinkPacket, SeedLinkPacketV3, StateDB};

const DEFAULT_HOSTNAME: &str = "localhost";
const PORT_RANGE: RangeInclusive<usize> = 1..=65535;

async fn write_xml<W: AsyncWrite + Unpin>(xml: String, writer: W) -> anyhow::Result<()> {
    let mut reader = Reader::from_str(&xml);
    reader.trim_text(true);

    // XXX(damb): at the time being quick_xml doesn't support indendation for AsyncWrite trait
    // implementations. For this reason, we fall back to a synchronous implementation.
    // see https://github.com/tafia/quick-xml/issues/605
    let mut writer = Writer::new_with_indent(writer, b' ', 4);

    loop {
        match reader.read_event()? {
            Event::Eof => {
                break;
            }
            e => {
                writer.write_event_async(e).await?;
            }
        }
    }

    Ok(())
}

/// Parses and validates the given port number.
fn port(s: &str) -> Result<u16, String> {
    let port: usize = s.parse().map_err(|_| format!("invalid port number"))?;
    if PORT_RANGE.contains(&port) {
        Ok(port as u16)
    } else {
        Err(format!(
            "invalid port number: not in range {}-{}",
            PORT_RANGE.start(),
            PORT_RANGE.end()
        ))
    }
}

/// Parses and validates the given duration.
fn keep_alive_interval(s: &str) -> Result<Duration, String> {
    let secs = s
        .parse::<u64>()
        .map_err(|_| format!("invalid value for keepalive interval"))?;
    let rv = Duration::from_secs(secs);
    if rv.is_zero() {
        return Err(format!("keepalive interval must be non-zero"));
    }

    Ok(rv)
}

// TODO(damb):
// - handle network timeout (-> must be handled by the client)
// - allow the user to force the seedlink protocol version used
// - Print packet header details (`-p` flag)
// - Unpack packet samples (`-u` flag)

#[derive(Debug, Clone, ValueEnum)]
enum InfoItem {
    /// Info type ID
    Id,
    /// Info type STATIONS
    Stations,
    /// Info type STREAMS
    Streams,
    /// Info type CONNECTIONS
    Connections,
}

#[derive(Parser)]
#[command(name = "slink-tool")]
#[command(version = "0.1")]
#[command(about = "Rust slinktool port", long_about=None)]
struct Args {
    /// SeedLink server hostname.
    #[arg(default_value_t = DEFAULT_HOSTNAME.to_string())]
    hostname: String,

    /// SeedLink server port.
    #[arg(default_value_t = DEFAULT_PORT)]
    #[arg(value_parser = port)]
    port: u16,

    /// Ping the server, report the server identifier and exit.
    #[arg(short = 'P', long)]
    ping: bool,

    /// Send keepalive (heartbeat) packets this often (seconds).
    #[arg(short = 'k', long = "keepalive", value_name = "SECONDS")]
    #[arg(value_parser = keep_alive_interval)]
    keep_alive: Option<Duration>,

    /// Save and restore stream state information to and from this file
    #[arg(short = 'x', long = "state-db", value_name = "FILE")]
    state_db: Option<PathBuf>,

    /// Configure the connection in dial-up mode.
    #[arg(short = 'd', long = "dial-up")]
    dial_up: bool,

    /// Enable pipelining by batching SeedLink commands.
    #[arg(short = 'b', long = "batch")]
    batch: bool,

    // TODO(damb):
    // - parse directly into stream_config and validate on the fly
    /// Define a comma-separated stream list for multi-station mode.
    ///
    /// STREAMS uses the following format: STREAM_1[:SELECTORS_1][,STREAM_2[:SELECTORS_2][,...]],
    /// where STREAM_i is in NET_STA format, e.g. 'IU_KONO:BHE BHN,GE_WLF,MN_AQU:HH?.D'
    #[arg(short = 'S', long, value_delimiter = ',', value_name = "STREAMS")]
    streams: Option<Vec<String>>,

    /// Write all received records to FILE.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<PathBuf>,

    /// Request information of type TYPE (case insensitive)
    #[arg(value_enum)]
    #[arg(short = 'i', long = "info", ignore_case = true, value_name = "TYPE")]
    info: Option<InfoItem>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let url = format!("slink://{}:{}", args.hostname, args.port);
    let client = Client::open(url).unwrap();
    let mut con = client
        .get_connection_with_timeout(Duration::from_secs(2))
        .await
        .unwrap();

    if args.ping {
        let resp = con.greet_raw().await.unwrap();
        for line in resp {
            println!("{}", line);
        }

        con.shutdown().await.unwrap();
        return;
    }

    let mut state_db = {
        if let Some(p) = args.state_db {
            Some(StateDB::open(p).await.unwrap())
        } else {
            None
        }
    };

    con.greet_raw().await.unwrap();

    if let Some(item) = args.info {
        match item {
            InfoItem::Id => {
                info!("requesting INFO type ID");
                match con.request_id_info_raw().await {
                    Ok(resp) => {
                        if con.protocol_version() == 3 {
                            write_xml(resp, io::stdout()).await.unwrap();
                            println!();
                        }
                    }
                    Err(e) => {
                        warn!("failed to download info of type ID ({})", e);
                    }
                }
            }
            InfoItem::Stations => {
                info!("requesting INFO type STATIONS");
                match con.request_station_info_raw().await {
                    Ok(resp) => {
                        if con.protocol_version() == 3 {
                            write_xml(resp, io::stdout()).await.unwrap();
                            println!();
                        }
                    }
                    Err(e) => {
                        warn!("failed to download info of type STATIONS ({})", e);
                    }
                }
            }
            InfoItem::Streams => {
                info!("requesting INFO type STREAMS");
                match con.request_stream_info_raw().await {
                    Ok(resp) => {
                        if con.protocol_version() == 3 {
                            write_xml(resp, io::stdout()).await.unwrap();
                            println!();
                        }
                    }
                    Err(e) => {
                        warn!("failed to download info of type STREAMS ({})", e);
                    }
                }
            }
            InfoItem::Connections => {
                info!("requesting INFO type CONNECTIONS");
                match con.request_connection_info_raw().await {
                    Ok(resp) => {
                        if con.protocol_version() == 3 {
                            write_xml(resp, io::stdout()).await.unwrap();
                            println!();
                        }
                    }
                    Err(e) => {
                        warn!("failed to download info of type CONNECTIONS ({})", e);
                    }
                }
            }
        }
    }

    if let Some(streams) = args.streams {
        for stream in streams {
            let split: Vec<&str> = stream.splitn(2, ':').collect();

            let mut selectors: Option<Vec<&str>> = None;
            if split.len() == 2 {
                selectors = Some(split[1].split(' ').collect());
            }

            let net_sta = split[0];
            let split_net_sta: Vec<&str> = net_sta.splitn(2, '_').collect();
            if split_net_sta.len() != 2 {
                panic!("invalid stream configuration: NET_STA");
            }

            let net_code = split_net_sta[0];
            let sta_code = split_net_sta[1];
            info!("[{}] requesting next available data", net_sta);
            con.add_stream(net_code, sta_code, &None, &None, &None)
                .unwrap();

            if let Some(selectors) = selectors {
                for selector in selectors {
                    con.add_stream(
                        net_code,
                        sta_code,
                        &Some(selector.to_string()),
                        &None,
                        &None,
                    )
                    .unwrap();
                }
            }
        }
    } else {
        con.shutdown().await.unwrap();
        return;
    }

    if let Some(ref mut state_db) = state_db {
        con.recover_state(state_db, false).await.unwrap();
    }

    let data_transfer_mode;
    if args.dial_up {
        data_transfer_mode = DataTransferMode::DialUp;
    } else {
        data_transfer_mode = DataTransferMode::RealTime;
    }

    con.configure(data_transfer_mode, None, args.batch)
        .await
        .unwrap();

    let mut ofs_dump;
    if let Some(output) = args.output {
        ofs_dump = Some(
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(output)
                .await
                .unwrap(),
        );
    } else {
        ofs_dump = None;
    }

    let packet_stream = con.packets(args.keep_alive);

    tokio::pin!(packet_stream);

    while let Some(ref packet) = packet_stream.try_next().await.unwrap() {
        match packet {
            SeedLinkPacket::V3(packet) => match packet {
                SeedLinkPacketV3::GenericData(packet) => {
                    let seq_num = packet.sequence_number().unwrap();
                    println!("seq {}", seq_num);
                    if let Some(ref mut ofs) = ofs_dump {
                        // dump to file
                        ofs.write(packet.raw_payload()).await.unwrap();
                    }

                    if let Some(ref mut state_db) = state_db {
                        let ms_record = packet.payload(MSControlFlags::empty()).unwrap();
                        let sid = ms_record.sid().unwrap();

                        state_db.store(&sid, seq_num as i64).await.unwrap();
                    }
                }
                SeedLinkPacketV3::Info(_) => {
                    // ignore keepalive packets
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn verify_cli() {
        use super::Args;
        use clap::CommandFactory;

        Args::command().debug_assert()
    }
}
