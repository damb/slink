use std::io;

use tracing::{debug, instrument};

use super::super::cmd::{Command, Data, Fetch, Select, Station, Time};
use super::FramedConnectionV3;

use crate::{Frame, SeedLinkDataTransferModeV3, SeedLinkError, SeedLinkResult, StreamConfig};

pub(crate) struct Negotiator<'a> {
    pub stream_config: &'a StreamConfig,
}

impl<'a> Negotiator<'a> {
    /// Configures the remote peer SeedLink server with `stream_config`.
    #[instrument(skip(self))]
    pub(crate) async fn negotiate(
        &self,
        connection: &mut FramedConnectionV3,
        data_transfer_mode: &SeedLinkDataTransferModeV3,
    ) -> SeedLinkResult<bool> {
        let cmd = Command::Station(Station::new(
            &self.stream_config.station,
            Some(self.stream_config.network.clone()),
        ));
        let frame = cmd.into_frame();

        debug!("sending command: '{}'", cmd);
        connection.write_frame(&frame).await?;

        if connection.batch_cmd_mode() {
            self.negotiate_streams(connection).await?;
            self.negotiate_data_transfer_mode(connection, data_transfer_mode)
                .await?;

            return Ok(true);
        }

        match connection.read_frame().await? {
            Frame::Ok => {
                debug!(
                    "response: station ({}_{}) is OK (station selected)",
                    self.stream_config.network, self.stream_config.station
                );

                self.negotiate_streams(connection).await?;
                self.negotiate_data_transfer_mode(connection, data_transfer_mode)
                    .await?
            }
            Frame::Error => {
                debug!(
                    "response: station ({}_{}) is ERROR (station omitted)",
                    self.stream_config.network, self.stream_config.station
                );
                return Ok(false);
            }
            frame => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "response: invalid response to command ({}): {:?}",
                        cmd, frame
                    ),
                )
                .into());
            }
        }

        Ok(true)
    }

    #[instrument(skip(self))]
    async fn negotiate_streams(&self, connection: &mut FramedConnectionV3) -> SeedLinkResult<()> {
        if self.stream_config.len() == 0 {
            return Ok(());
        }

        let mut accepted_sel_cnt = 0;
        for select_arg in self.stream_config.iter() {
            let cmd = Command::Select(Select::new(Some(select_arg.clone())));
            let frame = cmd.into_frame();

            debug!("sending command: '{}'", cmd);
            connection.write_frame(&frame).await?;

            if connection.batch_cmd_mode() {
                continue;
            }

            match connection.read_frame().await? {
                Frame::Ok => {
                    accepted_sel_cnt += 1;
                    debug!("response: select arg ({}) is OK (selected)", select_arg);
                }
                Frame::Error => {
                    debug!(
                        "response: select arg ({}) is ERROR (select arg omitted)",
                        select_arg
                    );
                }
                frame => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "response: invalid response to command ({}): {:?}",
                            cmd, frame
                        ),
                    )
                    .into());
                }
            }
        }

        if !connection.batch_cmd_mode() {
            debug!("number of accepted selectors: {}", accepted_sel_cnt);
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn negotiate_data_transfer_mode(
        &self,
        connection: &mut FramedConnectionV3,
        data_transfer_mode: &SeedLinkDataTransferModeV3,
    ) -> SeedLinkResult<()> {
        let cmd: Command;
        match data_transfer_mode {
            SeedLinkDataTransferModeV3::RealTime | SeedLinkDataTransferModeV3::DialUp => {
                let mut seq_num: Option<i32> = None;
                if let Some(seq_num_str) = &self.stream_config.seq_num {
                    seq_num = Some(
                        i32::from_str_radix(&seq_num_str, 16)
                            .map_err(|e| SeedLinkError::ClientError(e.to_string()))?,
                    );
                }

                if *data_transfer_mode == SeedLinkDataTransferModeV3::RealTime {
                    cmd = Command::Data(Data::new(seq_num, self.stream_config.time.clone()));
                } else {
                    cmd = Command::Fetch(Fetch::new(seq_num, self.stream_config.time.clone()));
                }
            }
            SeedLinkDataTransferModeV3::TimeWindow(t) => {
                cmd = Command::Time(Time::new(self.stream_config.time.clone(), Some(t.clone())));
            }
        }

        let frame = cmd.into_frame();

        debug!("sending action command: '{}'", cmd);
        connection.write_frame(&frame).await?;

        if connection.batch_cmd_mode() {
            return Ok(());
        }

        match connection.read_frame().await? {
            Frame::Ok => {
                debug!("response: action command successful");
            }
            Frame::Error => {
                return Err(SeedLinkError::ClientError(format!(
                    "response: action command not accepted: {}",
                    cmd
                )));
            }
            frame => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "response: invalid response to action command ({}): {:?}",
                        cmd, frame
                    ),
                )
                .into());
            }
        }

        Ok(())
    }
}
