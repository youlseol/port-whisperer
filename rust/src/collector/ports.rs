use anyhow::Result;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};

/// Returns (port, pid) pairs for all TCP LISTEN sockets.
pub fn get_listening_sockets() -> Result<Vec<(u16, u32)>> {
    let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let sockets = get_sockets_info(af, ProtocolFlags::TCP)?;

    let mut pairs: Vec<(u16, u32)> = sockets
        .into_iter()
        .filter(|s| {
            matches!(
                &s.protocol_socket_info,
                ProtocolSocketInfo::Tcp(tcp) if tcp.state == TcpState::Listen
            )
        })
        .filter_map(|s| {
            let pid = *s.associated_pids.first()?;
            let port = match &s.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => tcp.local_port,
                _ => return None,
            };
            Some((port, pid))
        })
        .collect();

    // Deduplicate by port (IPv4 + IPv6 dual-stack shows same port twice)
    pairs.sort_by_key(|(port, _)| *port);
    pairs.dedup_by_key(|(port, _)| *port);
    Ok(pairs)
}
