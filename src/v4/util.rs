use crate::IdInfoV4;

/// Returns the first line in response to the `HELLO` command.
///
/// Note that `protocol_versions` must be sorted in descending order.
pub fn to_first_hello_resp_line(
    implementation: &str,
    implementation_version: &str,
    protocol_versions: &Vec<(u8, u8)>,
    capabilities: &Option<Vec<String>>,
) -> String {
    assert!(!protocol_versions.is_empty());

    let slproto_str = protocol_versions
        .iter()
        .rev()
        .map(|v| format!("SLPROTO:{}.{}", v.0, v.1))
        .collect::<Vec<_>>()
        .join(" ");

    let mut line =
        format!("SeedLink v{highest_proto_major}.{highest_proto_minor} ({implementation}/{implementation_version}) :: {slproto_str}", highest_proto_major = protocol_versions[0].0, highest_proto_minor = protocol_versions[0].1);
    if let Some(capabilities) = capabilities {
        line += &format!(" {}", capabilities.join(" "))
    }

    line
}

/// Creates a `INFO ID` response object.
///
/// Note that `protocol_versions` must be sorted in descending order.
pub fn to_id_info(
    implementation: &str,
    implementation_version: &str,
    protocol_versions: &Vec<(u8, u8)>,
    data_center_description: &str,
    capabilities: &Option<Vec<String>>,
) -> IdInfoV4 {
    IdInfoV4 {
        software: to_first_hello_resp_line(
            implementation,
            implementation_version,
            protocol_versions,
            capabilities,
        ),
        organization: data_center_description.to_string(),
    }
}
