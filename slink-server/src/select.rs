use std::convert::From;
use std::ops::Deref;

use regex::{Error, Regex};
use time::OffsetDateTime;

use slink::{Format, SequenceNumberV4, Station, StationId, Stream, StreamId, SubFormat};

/// Station selection.
#[derive(Debug, Clone)]
pub struct StationSelect {
    id: StationId,

    seq_num: SequenceNumberV4,

    streams: Vec<StreamSelect>,
}

impl StationSelect {
    /// Returns the network code.
    pub fn net_code(&self) -> &str {
        self.id.net_code()
    }

    /// Returns the station code.
    pub fn sta_code(&self) -> &str {
        self.id.sta_code()
    }

    /// Returns the sequence number.
    pub fn seq_num(&self) -> &SequenceNumberV4 {
        &self.seq_num
    }

    /// Returns whether there are selected stream selects.
    pub fn has_selected(&self) -> bool {
        self.streams.iter().any(|s| s.selected)
    }

    /// Selects all stream selects.
    pub fn select_all(&mut self) {
        for stream_select in self.streams.iter_mut() {
            stream_select.selected = true;
        }
    }

    /// Deselects all stream selects.
    pub fn select_none(&mut self) {
        for stream_select in self.streams.iter_mut() {
            stream_select.selected = false;
        }
    }
}

impl From<Station> for StationSelect {
    fn from(item: Station) -> Self {
        let mut streams = Vec::new();
        for st in item.iter() {
            streams.push(StreamSelect::from(st.clone()))
        }

        Self {
            id: item.id().clone(),
            seq_num: SequenceNumberV4::Number(item.start_seq()),
            streams,
        }
    }
}

impl Deref for StationSelect {
    type Target = Vec<StreamSelect>;

    fn deref(&self) -> &Self::Target {
        &self.streams
    }
}

/// Stream selection.
#[derive(Debug, Clone)]
pub struct StreamSelect {
    selected: bool,
    excluded: bool,

    id: StreamId,

    format: Format,
    subformat: SubFormat,

    start_time: Option<OffsetDateTime>,
    end_time: Option<OffsetDateTime>,

    filter: Option<String>,
}

impl StreamSelect {
    /// Returns the location code.
    pub fn loc_code(&self) -> &str {
        self.id.loc_code()
    }

    /// Returns the band code.
    pub fn band_code(&self) -> &str {
        self.id.band_code()
    }

    /// Returns the source code.
    pub fn source_code(&self) -> &str {
        self.id.source_code()
    }

    /// Returns the subsource code.
    pub fn subsource_code(&self) -> &str {
        self.id.subsource_code()
    }

    /// Returns the format.
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// Returns the subformat.
    pub fn subformat(&self) -> &SubFormat {
        &self.subformat
    }

    /// Returns the start time.
    pub fn start_time(&self) -> &Option<OffsetDateTime> {
        &self.start_time
    }

    /// Returns the end time.
    pub fn end_time(&self) -> &Option<OffsetDateTime> {
        &self.end_time
    }

    /// Returns the filter property.
    pub fn filter(&self) -> &Option<String> {
        &self.filter
    }

    /// Returns whether the stream select is selected.
    pub fn is_selected(&self) -> bool {
        self.selected && !self.excluded
    }
}

impl From<Stream> for StreamSelect {
    fn from(item: Stream) -> Self {
        Self {
            selected: true,
            excluded: false,
            id: item.id().clone(),
            format: item.format().clone(),
            subformat: item.subformat().clone(),
            start_time: None,
            end_time: None,
            filter: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Select(Vec<StationSelect>);

impl Select {
    /// Creates a new `Select` from stations.
    pub fn new(stations: Vec<Station>) -> Self {
        let select = stations.into_iter().map(|sta| sta.into()).collect();

        Self(select)
    }

    /// Creates a new `Select` from stations matching pattern.
    pub fn with_pattern(stations: &Vec<Station>, station_pattern: &str) -> Self {
        let re = create_regex(station_pattern).unwrap();

        let mut select = Vec::new();
        for sta in stations.iter() {
            let station_id = station_id(sta.net_code(), sta.sta_code());
            if re.is_match(&station_id) {
                select.push(sta.clone().into());
            }
        }

        Self(select)
    }

    /// Returns whether there are any selected stations.
    pub fn has_selected(&self) -> bool {
        self.0.iter().any(|s| s.has_selected())
    }

    /// Selects all station selects.
    pub fn select_all(&mut self) {
        for sta_select in self.0.iter_mut() {
            sta_select.select_all();
        }
    }

    /// Deselects all stream selects.
    pub fn select_none(&mut self) {
        for sta_select in self.0.iter_mut() {
            sta_select.select_none();
        }
    }

    /// Applies rules to the selection.
    pub fn apply(
        &mut self,
        exclude: bool,
        stream_pattern: &str,
        format_subformat_pattern: &Option<String>,
        filter: &Option<String>,
    ) {
        assert!(filter.is_none() || (filter.is_some() && !exclude));

        let stream_re = create_regex(stream_pattern).unwrap();

        let format_subformat_re = if let Some(ref pattern) = format_subformat_pattern {
            Some(create_regex(pattern).unwrap())
        } else {
            None
        };

        for sta_select in self.0.iter_mut() {
            for stream_select in sta_select.streams.iter_mut() {
                let stream_id = stream_select.id.to_string();

                if stream_re.is_match(&stream_id) {
                    if let Some(ref format_subformat_re) = format_subformat_re {
                        let format_subformat =
                            format!("{}{}", stream_select.format, stream_select.subformat);
                        if format_subformat_re.is_match(&format_subformat) {
                            if exclude {
                                stream_select.excluded = true;
                            } else {
                                stream_select.selected = true;
                                if stream_select.filter.is_none() {
                                    stream_select.filter = filter.clone();
                                }
                            }
                        }
                    } else if exclude {
                        stream_select.excluded = true;
                    } else {
                        stream_select.selected = true;
                        if stream_select.filter.is_none() {
                            stream_select.filter = filter.clone();
                        }
                    }
                }
            }
        }
    }

    /// Sets the sequence number for selected stations.
    pub fn set_seq_num(&mut self, seq_num: &SequenceNumberV4) {
        for sta_select in self.0.iter_mut() {
            if !sta_select.has_selected() {
                continue;
            }

            match seq_num {
                SequenceNumberV4::All | SequenceNumberV4::Next => {
                    sta_select.seq_num = seq_num.clone();
                }
                SequenceNumberV4::Number(num) => {
                    let orig_num = sta_select.seq_num();
                    match orig_num {
                        SequenceNumberV4::Number(orig_num) => {
                            if num > orig_num {
                                sta_select.seq_num = seq_num.clone();
                            }
                        }
                        _ => {}
                    }
                }
            };
        }
    }

    /// Sets the time window for selected streams.
    pub fn set_time(&mut self, start_time: &OffsetDateTime, end_time: &Option<OffsetDateTime>) {
        for sta_select in self.0.iter_mut() {
            for stream_select in sta_select.streams.iter_mut() {
                if !stream_select.is_selected() {
                    continue;
                }

                stream_select.start_time = Some(start_time.clone());
                stream_select.end_time = end_time.clone();
            }
        }
    }
}

impl Deref for Select {
    type Target = Vec<StationSelect>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Creates a regex from a pattern.
fn create_regex(pattern: &str) -> Result<Regex, Error> {
    let pattern = pattern.replace('*', ".*");
    let pattern = pattern.replace('?', ".");
    Regex::new(&pattern)
}

/// Returns a compound station identifier.
fn station_id(network: &str, station: &str) -> String {
    format!("{}_{}", network, station)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn select_empty() {
        let select = Select::default();
        assert!(!select.has_selected());
    }

    #[test]
    fn select_single_station_no_streams() {
        todo!()
    }

    // TODO(damb): add more tests
}
