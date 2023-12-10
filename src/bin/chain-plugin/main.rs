use nix::sys::stat::Mode;
use nix::unistd;
use std::fs::File;
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use anyhow::bail;
use daemonize::Daemonize;
use futures::TryStreamExt;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
// use tokio::net::unix::pipe;
use tracing::{debug, error};
use tracing_subscriber;

use clap::Parser;

use slink::{Client, DataTransferMode, SeedLinkPacket, SeedLinkPacketV3};

const DEFAULT_PATH_FIFO: &str = "/var/tmp/slink/plugin.fifo";

fn fifo(s: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(s);
    if p.is_absolute() {
        Ok(p)
    } else {
        Err("invalid path: must be absolute".to_string())
    }
}

fn slink_url(url: &str) -> Result<String, String> {
    if let Err(e) = Client::open(url) {
        return Err(e.to_string());
    }

    Ok(url.to_string())
}

// TODO(damb):
// - handle network timeout
// - handle different SeedLink protocol versions (allow the user to force the protocol version
// specified)
// - allow plugin to be configured via with a configuration file (see: https://crates.io/crates/clap-serde-derive/)

#[derive(Parser)]
#[command(name = "chain-plugin")]
#[command(version = "0.1")]
#[command(about = "slink chain-plugin", long_about=None)]
struct Args {
    /// FIFO (named pipe) path SeedLink packets are written to
    #[arg(default_value = DEFAULT_PATH_FIFO)]
    #[arg(value_name = "FIFO")]
    #[arg(short = 'o', long)]
    #[arg(value_parser = fifo)]
    fifo: PathBuf,

    /// SeedLink server URL e.g. slink://host[:port]
    #[arg(value_name = "URL")]
    #[arg(value_parser = slink_url)]
    url: String,

    // TODO(damb):
    // - parse directly into stream_config and validate on the fly
    /// Define a comma-separated stream list for multi-station mode. STREAMS uses the following
    /// format: STREAM_1[:SELECTORS_1][,STREAM_2[:SELECTORS_2][,...]], where STREAM_i is in NET_STA
    /// format, e.g. 'IU_KONO:BHE BHN,GE_WLF,MN_AQU:HH?.D'.
    /// If not specified, all streams available are requested.
    #[arg(short = 'S', long, value_delimiter = ',', value_name = "STREAMS")]
    streams: Option<Vec<String>>,

    /// Enable pipelining by batching SeedLink commands.
    #[arg(short = 'b', long = "batch")]
    batch: bool,

    /// Run as daemon
    #[arg(short = 'D', long)]
    daemonize: bool,
}

#[tokio::main]
async fn tokio_main(args: &Args) -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::open(args.url.clone())?;
    let mut con = client
        .get_connection_with_timeout(Duration::from_secs(2))
        .await?;

    con.greet_raw().await?;

    if let Some(streams) = &args.streams {
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
            con.add_stream(net_code, sta_code, &None, &None, &None)?;

            if let Some(selectors) = selectors {
                for selector in selectors {
                    con.add_stream(
                        net_code,
                        sta_code,
                        &Some(selector.to_string()),
                        &None,
                        &None,
                    )?;
                }
            }
        }
    }

    con.configure(DataTransferMode::RealTime, None, args.batch)
        .await
        .unwrap();

    // create fifo directory
    if let Some(fifo_dir) = args.fifo.parent() {
        if !fifo_dir.is_dir() {
            fs::create_dir_all(fifo_dir).await?;
        }
    }

    if let Ok(attr) = fs::metadata(&args.fifo).await {
        let file_type = attr.file_type();
        if !file_type.is_fifo() {
            bail!("failed to create fifo, existing path with incompatible file type");
        }
    } else {
        unistd::mkfifo(&args.fifo, Mode::S_IRWXU)?;
    }

    // let mut tx = pipe::OpenOptions::new()
    //     .read_write(true)
    //     .unchecked(true)
    //     .open_sender(&args.fifo)?;
    let mut tx = OpenOptions::new().write(true).open(&args.fifo).await?;

    // TODO(damb): send keepalive packets
    let packet_stream = con.packets(None);

    tokio::pin!(packet_stream);

    while let Some(packet) = packet_stream.try_next().await? {
        match &packet {
            SeedLinkPacket::V3(packet) => {
                match &packet {
                    SeedLinkPacketV3::GenericData(packet) => {
                        debug!("received packet: seq {}", packet.sequence_number()?);
                        tx.write(packet.raw()).await?;
                    }
                    _ => {
                        debug!("received info packet");
                        // ignore
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.daemonize {
        let stdout = File::create("/tmp/daemon.out").unwrap();
        let stderr = File::create("/tmp/daemon.err").unwrap();
        let daemonize = Daemonize::new()
            // .user("nobody")
            .stdout(stdout)
            .stderr(stderr);

        if let Err(e) = daemonize.start() {
            error!("failed to daemonize plugin ({})", e);
            process::exit(2);
        }
    }

    tokio_main(&args)
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
