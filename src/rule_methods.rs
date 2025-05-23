use std::ffi::CString;
use std::net::IpAddr;

use ipnetwork::IpNetwork;

use crate::data_type::ip_to_vec;
use crate::error::BuilderError;
use crate::expr::ct::{ConnTrackState, Conntrack, ConntrackKey};
use crate::expr::{
    Bitwise, Byteorder, ByteorderOp, Cmp, CmpOp, ExtHdr, ExtHdrOp, HighLevelPayload,
    IPv4HeaderField, IPv6HeaderField, Immediate, Masquerade, Meta, MetaType, Nat, NatType,
    NetworkHeaderField, Payload, Register, Rt, RtKey, TCPHeaderField, TransportHeaderField,
    UDPHeaderField, VerdictKind,
};
use crate::sys::NFT_PAYLOAD_TRANSPORT_HEADER;
use crate::{ProtocolFamily, Rule};

/// Simple protocol description. Note that it does not implement other layer 4 protocols as
/// IGMP et al. See [`Rule::igmp`] for a workaround.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Protocol {
    TCP,
    UDP,
}

impl Rule {
    fn match_port(mut self, port: u16, protocol: Protocol, source: bool) -> Self {
        self = self.protocol(protocol);
        self.add_expr(
            HighLevelPayload::Transport(match protocol {
                Protocol::TCP => TransportHeaderField::Tcp(if source {
                    TCPHeaderField::Sport
                } else {
                    TCPHeaderField::Dport
                }),
                Protocol::UDP => TransportHeaderField::Udp(if source {
                    UDPHeaderField::Sport
                } else {
                    UDPHeaderField::Dport
                }),
            })
            .build(),
        );
        self.add_expr(Cmp::new(CmpOp::Eq, port.to_be_bytes()));
        self
    }

    pub fn match_ip(mut self, ip: IpAddr, source: bool) -> Self {
        self.add_expr(Meta::new(MetaType::NfProto));
        match ip {
            IpAddr::V4(addr) => {
                self.add_expr(Cmp::new(CmpOp::Eq, [libc::NFPROTO_IPV4 as u8]));
                self.add_expr(
                    HighLevelPayload::Network(NetworkHeaderField::IPv4(if source {
                        IPv4HeaderField::Saddr
                    } else {
                        IPv4HeaderField::Daddr
                    }))
                    .build(),
                );
                self.add_expr(Cmp::new(CmpOp::Eq, addr.octets()));
            }
            IpAddr::V6(addr) => {
                self.add_expr(Cmp::new(CmpOp::Eq, [libc::NFPROTO_IPV6 as u8]));
                self.add_expr(
                    HighLevelPayload::Network(NetworkHeaderField::IPv6(if source {
                        IPv6HeaderField::Saddr
                    } else {
                        IPv6HeaderField::Daddr
                    }))
                    .build(),
                );
                self.add_expr(Cmp::new(CmpOp::Eq, addr.octets()));
            }
        }
        self
    }

    pub fn match_network(mut self, net: IpNetwork, source: bool) -> Result<Self, BuilderError> {
        self.add_expr(Meta::new(MetaType::NfProto));
        match net {
            IpNetwork::V4(_) => {
                self.add_expr(Cmp::new(CmpOp::Eq, [libc::NFPROTO_IPV4 as u8]));
                self.add_expr(
                    HighLevelPayload::Network(NetworkHeaderField::IPv4(if source {
                        IPv4HeaderField::Saddr
                    } else {
                        IPv4HeaderField::Daddr
                    }))
                    .build(),
                );
                self.add_expr(Bitwise::new(ip_to_vec(net.mask()), 0u32.to_be_bytes())?);
            }
            IpNetwork::V6(_) => {
                self.add_expr(Cmp::new(CmpOp::Eq, [libc::NFPROTO_IPV6 as u8]));
                self.add_expr(
                    HighLevelPayload::Network(NetworkHeaderField::IPv6(if source {
                        IPv6HeaderField::Saddr
                    } else {
                        IPv6HeaderField::Daddr
                    }))
                    .build(),
                );
                self.add_expr(Bitwise::new(ip_to_vec(net.mask()), 0u128.to_be_bytes())?);
            }
        }
        self.add_expr(Cmp::new(CmpOp::Eq, ip_to_vec(net.network())));
        Ok(self)
    }
}

impl Rule {
    /// Matches ICMP packets.
    pub fn icmp(mut self) -> Self {
        self.add_expr(Meta::new(MetaType::L4Proto));
        self.add_expr(Cmp::new(CmpOp::Eq, [libc::IPPROTO_ICMP as u8]));
        self
    }
    /// Matches ICMPv6 packets.
    pub fn icmpv6(mut self) -> Self {
        self.add_expr(Meta::new(MetaType::L4Proto));
        self.add_expr(Cmp::new(CmpOp::Eq, [libc::IPPROTO_ICMPV6 as u8]));
        self
    }
    /// Matches IGMP packets.
    pub fn igmp(mut self) -> Self {
        self.add_expr(Meta::new(MetaType::L4Proto));
        self.add_expr(Cmp::new(CmpOp::Eq, [libc::IPPROTO_IGMP as u8]));
        self
    }
    /// Matches 4in6 packets.
    pub fn ip4in6(mut self) -> Self {
        self.add_expr(Meta::new(MetaType::NfProto));
        self.add_expr(Cmp::new(CmpOp::Eq, [libc::NFPROTO_IPV6 as u8]));
        self.add_expr(
            HighLevelPayload::Network(NetworkHeaderField::IPv6(IPv6HeaderField::NextHeader))
                .build(),
        );
        self.add_expr(Cmp::new(CmpOp::Eq, [60 as u8]));
        self
    }
    /// Matches 6in4 packets.
    pub fn ip6in4(mut self) -> Self {
        self.add_expr(Meta::new(MetaType::NfProto));
        self.add_expr(Cmp::new(CmpOp::Eq, [libc::NFPROTO_IPV4 as u8]));
        self.add_expr(
            HighLevelPayload::Network(NetworkHeaderField::IPv4(IPv4HeaderField::Protocol)).build(),
        );
        self.add_expr(Cmp::new(CmpOp::Eq, [41 as u8]));
        self
    }
    /// Matches packets from source `port` and `protocol`.
    pub fn sport(self, port: u16, protocol: Protocol) -> Self {
        self.match_port(port, protocol, false)
    }
    /// Matches packets to destination `port` and `protocol`.
    pub fn dport(self, port: u16, protocol: Protocol) -> Self {
        self.match_port(port, protocol, false)
    }
    /// Matches packets on `protocol`.
    pub fn protocol(mut self, protocol: Protocol) -> Self {
        self.add_expr(Meta::new(MetaType::L4Proto));
        self.add_expr(Cmp::new(
            CmpOp::Eq,
            [match protocol {
                Protocol::TCP => libc::IPPROTO_TCP,
                Protocol::UDP => libc::IPPROTO_UDP,
            } as u8],
        ));
        self
    }
    /// Matches packets in an already established connection.
    pub fn established(mut self) -> Result<Self, BuilderError> {
        let allowed_states = ConnTrackState::ESTABLISHED.bits();
        self.add_expr(Conntrack::new(ConntrackKey::State));
        self.add_expr(Bitwise::new(
            allowed_states.to_le_bytes(),
            0u32.to_be_bytes(),
        )?);
        self.add_expr(Cmp::new(CmpOp::Neq, 0u32.to_be_bytes()));
        Ok(self)
    }
    /// Matches packets going through `iface_index`. Interface indexes can be queried with
    /// `iface_index()`.
    pub fn iface_id(mut self, iface_index: libc::c_uint) -> Self {
        self.add_expr(Meta::new(MetaType::Iif));
        self.add_expr(Cmp::new(CmpOp::Eq, iface_index.to_be_bytes()));
        self
    }
    /// Matches packets going through `iface_name`, an interface name, as in "wlan0" or "lo"
    pub fn iface(mut self, iface_name: &str) -> Result<Self, BuilderError> {
        if iface_name.len() >= libc::IFNAMSIZ {
            return Err(BuilderError::InterfaceNameTooLong);
        }
        let mut iface_vec = iface_name.as_bytes().to_vec();
        // null terminator
        iface_vec.push(0u8);

        self.add_expr(Meta::new(MetaType::IifName));
        self.add_expr(Cmp::new(CmpOp::Eq, iface_vec));
        Ok(self)
    }
    /// Matches packets leaving through `oface_index`. Interface indexes can be queried with
    /// `iface_index()`.
    pub fn oface_id(mut self, oface_index: libc::c_uint) -> Self {
        self.add_expr(Meta::new(MetaType::Oif));
        self.add_expr(Cmp::new(CmpOp::Eq, oface_index.to_be_bytes()));
        self
    }
    /// Matches packets leaving through `oface_name`, an interface name, as in "wlan0" or "lo"
    pub fn oface(mut self, oface_name: &str) -> Result<Self, BuilderError> {
        if oface_name.len() >= libc::IFNAMSIZ {
            return Err(BuilderError::InterfaceNameTooLong);
        }
        let mut oface_vec = oface_name.as_bytes().to_vec();
        // null terminator
        oface_vec.push(0u8);

        self.add_expr(Meta::new(MetaType::OifName));
        self.add_expr(Cmp::new(CmpOp::Eq, oface_vec));
        Ok(self)
    }
    /// Matches packets whose source IP address is `saddr`.
    pub fn saddr(self, ip: IpAddr) -> Self {
        self.match_ip(ip, true)
    }
    /// Matches packets whose destination IP address is `saddr`.
    pub fn daddr(self, ip: IpAddr) -> Self {
        self.match_ip(ip, false)
    }
    /// Matches packets whose source network is `net`.
    pub fn snetwork(self, net: IpNetwork) -> Result<Self, BuilderError> {
        self.match_network(net, true)
    }
    /// Matches packets whose destination network is `net`.
    pub fn dnetwork(self, net: IpNetwork) -> Result<Self, BuilderError> {
        self.match_network(net, false)
    }
    /// Adds the `Accept` verdict to the rule. The packet will be sent to destination.
    pub fn accept(mut self) -> Self {
        self.add_expr(Immediate::new_verdict(VerdictKind::Accept));
        self
    }
    /// Adds the `Drop` verdict to the rule. The packet will be dropped.
    pub fn drop(mut self) -> Self {
        self.add_expr(Immediate::new_verdict(VerdictKind::Drop));
        self
    }
    /// Adds the `Masquerade` verdict to the rule. The packet will have its
    /// source address rewritten.
    pub fn masquerade(mut self) -> Self {
        self.add_expr(Masquerade {});
        self
    }
    /// Adds the `Nat` verdict to the rule, with type `DNat`. The packet
    /// will have its destination address and optionally port rewritten.
    pub fn dnat(mut self, dst: IpAddr, port: Option<u16>) -> Self {
        self.add_expr(Immediate::new_data(ip_to_vec(dst), Register::Reg1));
        if let Some(port) = port {
            self.add_expr(Immediate::new_data(
                port.to_be_bytes().to_vec(),
                Register::Reg2,
            ));
        }
        self.add_expr(Nat {
            nat_type: Some(NatType::DNat),
            family: Some(ProtocolFamily::Ipv4),
            ip_register: Some(Register::Reg1),
            port_register: port.map(|_| Register::Reg2),
        });
        self
    }
    /// Adds the `ExtHdr` expression to the rule. The packet will have
    /// its MSS rewritten.
    pub fn set_mss(mut self, mss: u16) -> Self {
        self.add_expr(Immediate::new_data(
            mss.to_be_bytes().to_vec(),
            Register::Reg1,
        ));
        self.add_expr(
            ExtHdr::default()
                .with_sreg(Register::Reg1)
                .with_typ(2u8)
                .with_offset(2u32)
                .with_len(2u32)
                .with_op(ExtHdrOp::TCPOpt),
        );
        self
    }
    /// Sets the TCP MSS to the path MTU observed by the routing cache.
    pub fn clamp_mss_to_pmtu(mut self) -> Self {
        self.add_expr(
            Rt::default()
                .with_dreg(Register::Reg1)
                .with_key(RtKey::TCPMSS),
        );
        self.add_expr(
            Byteorder::default()
                .with_sreg(Register::Reg1)
                .with_dreg(Register::Reg1)
                .with_op(ByteorderOp::HtoN)
                .with_len(2u32)
                .with_siz(2u32),
        );
        self.add_expr(
            ExtHdr::default()
                .with_sreg(Register::Reg1)
                .with_typ(2u8)
                .with_offset(2u32)
                .with_len(2u32)
                .with_op(ExtHdrOp::TCPOpt),
        );
        self
    }
    /// Matches TCP packets whose flags include SYN.
    pub fn syn(mut self) -> Result<Self, BuilderError> {
        self.add_expr(
            Payload::default()
                .with_base(NFT_PAYLOAD_TRANSPORT_HEADER)
                .with_offset(13u32)
                .with_len(1u32)
                .with_dreg(Register::Reg1),
        );
        self.add_expr(Bitwise::new(2u8.to_be_bytes(), 0u8.to_be_bytes())?);
        self.add_expr(Cmp::new(CmpOp::Neq, 0u8.to_be_bytes()));
        Ok(self)
    }
}

/// Looks up the interface index for a given interface name.
pub fn iface_index(name: &str) -> Result<libc::c_uint, std::io::Error> {
    let c_name = CString::new(name)?;
    let index = unsafe { libc::if_nametoindex(c_name.as_ptr()) };
    match index {
        0 => Err(std::io::Error::last_os_error()),
        _ => Ok(index),
    }
}
