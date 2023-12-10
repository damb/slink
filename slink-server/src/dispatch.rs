use std::io;

use slink::{CommandV4, InfoCmdItemV4, InfoV4, ProtocolErrorV4};

use crate::client::{ClientHandle, FromServer};
use crate::negotiate::StationNegotiator;
use crate::response::Hello;
use crate::select::Select;
use crate::util::to_id_info_v4;
use crate::{SeedLinkServer, HIGHEST_SUPPORTED_PROTO_VERSION};

#[derive(Clone, Debug, Default)]
pub struct Dispatcher<T> {
    server: T,
}

impl<T> Dispatcher<T> {
    pub fn new(mut service: T) -> Self {
        Self { server: service }
    }

    pub fn server(&self) -> &T {
        &self.server
    }

    pub fn server_mut(&mut self) -> &mut T {
        &mut self.server
    }
}

impl<T: SeedLinkServer> Dispatcher<T> {
    pub async fn dispatch(
        &mut self,
        cmd: &CommandV4,
        client_handle: &mut ClientHandle,
    ) -> Result<(), io::Error> {
        self.dispatch_v4(cmd, client_handle).await
    }

    async fn dispatch_v4(
        &mut self,
        cmd: &CommandV4,
        client_handle: &mut ClientHandle,
    ) -> Result<(), io::Error> {
        match cmd {
            CommandV4::Station(station_cmd) => {
                if client_handle.negotiator.is_some() {
                    client_handle.send(FromServer::Error(
                        ProtocolErrorV4::unexpected_command().to_string(),
                    ))?;
                    return Ok(());
                }

                let stations = self
                    .server()
                    .inventory_streams(&station_cmd.station_pattern, &None, &None)
                    .await;

                if let Err(err) = stations {
                    client_handle.send(FromServer::Error(err.to_string()))?;
                    return Ok(());
                }

                let select = Select::new(stations.unwrap().clone());
                client_handle.negotiator = Some(StationNegotiator::new(select));

                client_handle.send(FromServer::Ok)
            }
            CommandV4::Select(select_cmd) => {
                let res = if let Some(ref mut negotiator) = client_handle.negotiator {
                    negotiator.next(&CommandV4::Select(select_cmd.clone()))
                } else {
                    Err(ProtocolErrorV4::unexpected_command())
                };

                match res {
                    Ok(_) => client_handle.send(FromServer::Ok),
                    Err(err) => client_handle.send(FromServer::Error(err.to_string())),
                }
            }
            CommandV4::Data(data_cmd) => {
                let res = if let Some(ref mut negotiator) = client_handle.negotiator {
                    negotiator.next(&CommandV4::Data(data_cmd.clone()))
                } else {
                    Err(ProtocolErrorV4::unexpected_command())
                };

                match res {
                    Ok(_) => {
                        client_handle
                            .selects
                            .push(client_handle.negotiator.take().unwrap().select);
                        client_handle.send(FromServer::Ok)
                    }
                    Err(err) => client_handle.send(FromServer::Error(err.to_string())),
                }
            }
            CommandV4::End(end_cmd) => {
                // XXX(damb): go into streaming mode
                todo!()
            }
            CommandV4::EndFetch(endfetch_cmd) => {
                // XXX(damb): go into streaming mode
                todo!()
            }
            CommandV4::Hello(_) => {
                let hello = Hello {
                    implementation: self.server.implementation().to_string(),
                    implementation_version: self.server.implementation_version().to_string(),
                    data_center_description: self.server.data_center_description().to_string(),
                };

                client_handle.send(FromServer::Hello(hello))
            }
            CommandV4::Info(info_cmd) => match info_cmd.item {
                InfoCmdItemV4::Id => {
                    let id_info = to_id_info_v4(
                        self.server(),
                        &vec![(
                            HIGHEST_SUPPORTED_PROTO_VERSION.0,
                            HIGHEST_SUPPORTED_PROTO_VERSION.1,
                        )],
                        &None,
                    );

                    client_handle.send(FromServer::Info(InfoV4::Id(id_info)))
                }
                _ => {
                    todo!();
                }
            },
            _ => {
                // TODO
                Ok(())
            }
        }
    }
}
