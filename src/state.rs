use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};
use tokio::task;

use crate::{FDSNSourceId, SeedLinkError, SeedLinkResult};

/// Represents a state database for clients.
#[derive(Debug, Clone)]
pub struct StateDB {
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl StateDB {
    /// Creates a new `StateDB`.
    pub async fn open<P: AsRef<Path>>(p: P) -> SeedLinkResult<Self> {
        let p = p.as_ref().to_path_buf();
        let join = task::spawn_blocking(move || {
            let con = Connection::open(p)
                .map_err(|e| SeedLinkError::StateDBError(e.to_string()))
                .map_err(|e| {
                    SeedLinkError::StateDBError(format!(
                        "failed to open state db ({})",
                        e.to_string()
                    ))
                })?;

            con.execute(
                "CREATE TABLE IF NOT EXISTS stream (\
                    id INTEGER PRIMARY KEY, \
                    sid TEXT NOT NULL, \
                    seq BIGINT NOT NULL \
                )",
                (),
            )
            .map_err(|e| {
                SeedLinkError::StateDBError(format!(
                    "failed to initialize state db ({})",
                    e.to_string()
                ))
            })?;

            con.execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_stream_sid ON stream(sid)",
                (),
            )
            .map_err(|e| {
                SeedLinkError::StateDBError(format!(
                    "failed to initialize state db ({})",
                    e.to_string()
                ))
            })?;

            let rv: SeedLinkResult<Connection> = Ok(con);
            rv
        });

        let con = join
            .await
            .map_err(|e| SeedLinkError::StateDBError(e.to_string()))??;

        Ok(Self {
            con: Arc::new(Mutex::new(con)),
        })
    }

    /// Stores the sequence number `seq_num` associated with the stream identified by the
    /// `FDSNSourceId`.
    pub async fn store(&mut self, sid: &str, seq_num: i64) -> SeedLinkResult<usize> {
        let cloned_con = self.con.clone();

        let sid = sid.parse::<FDSNSourceId>()?;

        let join = task::spawn_blocking(move || {
            let con = cloned_con.lock().map_err(|e| {
                SeedLinkError::StateDBError(format!(
                    "failed to lock connection ({})",
                    e.to_string()
                ))
            })?;
            con.execute(
                "REPLACE INTO stream(sid, seq) VALUES(?1, ?2)",
                (sid.to_string(), seq_num),
            )
            .map_err(|e| {
                SeedLinkError::StateDBError(format!("failed to execute task ({})", e.to_string()))
            })
        });

        join.await
            .map_err(|e| SeedLinkError::StateDBError(e.to_string()))?
    }

    /// Returns the sequence number associated with station identified by the network code `net`
    /// and the station code `sta`.
    pub async fn seq_num(&mut self, sid: &str) -> SeedLinkResult<Option<i64>> {
        let cloned_con = self.con.clone();

        let sid = sid.parse::<FDSNSourceId>()?;

        let join = task::spawn_blocking(move || {
            let con = cloned_con.lock().map_err(|e| {
                SeedLinkError::StateDBError(format!(
                    "failed to lock connection ({})",
                    e.to_string()
                ))
            })?;
            let mut stmt = con
                .prepare("SELECT seq FROM stream WHERE sid=?1")
                .map_err(|e| {
                    SeedLinkError::StateDBError(format!(
                        "failed to prepare statement ({})",
                        e.to_string()
                    ))
                })?;
            let res: SeedLinkResult<Option<i64>> = stmt
                .query_row([sid.to_string()], |row| Ok(row.get(0).optional()?))
                .map_err(|e| {
                    SeedLinkError::StateDBError(format!(
                        "failed to execute query ({})",
                        e.to_string()
                    ))
                });
            res
        });

        join.await
            .map_err(|e| SeedLinkError::StateDBError(e.to_string()))?
    }

    /// Returns the complete state information available.
    pub async fn state(&mut self) -> SeedLinkResult<Vec<(FDSNSourceId, i64)>> {
        let cloned_con = self.con.clone();

        let join = task::spawn_blocking(move || {
            let con = cloned_con.lock().map_err(|e| {
                SeedLinkError::StateDBError(format!(
                    "failed to lock connection ({})",
                    e.to_string()
                ))
            })?;

            let mut stmt = con
                .prepare("SELECT sid, seq FROM stream ORDER BY sid")
                .map_err(|e| {
                    SeedLinkError::StateDBError(format!(
                        "failed to prepare statement ({})",
                        e.to_string()
                    ))
                })?;
            let rows = stmt
                .query_map([], |row| Self::convert_row(row.get(0)?, row.get(1)?))
                .map_err(|e| {
                    SeedLinkError::StateDBError(format!(
                        "failed to execute query ({})",
                        e.to_string()
                    ))
                })?;

            let mut rv = Vec::new();
            for res in rows {
                let (sid, seq) = res.map_err(|e| {
                    SeedLinkError::StateDBError(format!(
                        "error while executing query ({})",
                        e.to_string()
                    ))
                })?;
                rv.push((sid.parse::<FDSNSourceId>()?, seq));
            }

            Ok(rv)
        });

        join.await
            .map_err(|e| SeedLinkError::StateDBError(e.to_string()))?
    }

    fn convert_row(sid: String, seq: i64) -> rusqlite::Result<(String, i64)> {
        Ok((sid, seq))
    }
}
