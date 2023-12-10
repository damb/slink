use slink::IdInfoV4;

use crate::SeedLinkServer;

/// Returns an `INFO ID` response object.
///
/// Note that `protocol_versions` must be sorted in descending order.
pub fn to_id_info_v4(
    server: &impl SeedLinkServer,
    protocol_versions: &Vec<(u8, u8)>,
    capabilities: &Option<Vec<String>>,
) -> IdInfoV4 {
    slink::to_id_info_v4(
        server.implementation(),
        server.implementation_version(),
        protocol_versions,
        server.data_center_description(),
        capabilities,
    )
}

