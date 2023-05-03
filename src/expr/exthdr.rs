use rustables_macros::{nfnetlink_enum, nfnetlink_struct};

use crate::sys::{
    NFTA_EXTHDR_DREG, NFTA_EXTHDR_FLAGS, NFTA_EXTHDR_LEN, NFTA_EXTHDR_OFFSET, NFTA_EXTHDR_OP,
    NFTA_EXTHDR_SREG, NFTA_EXTHDR_TYPE, NFT_EXTHDR_OP_IPV6, NFT_EXTHDR_OP_TCPOPT,
};

use super::{Expression, Register};

/// Header operation.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[nfnetlink_enum(u32, nested = true)]
pub enum ExtHdrOp {
    /// IPv6.
    IPv6 = NFT_EXTHDR_OP_IPV6,
    /// TCP options.
    TCPOpt = NFT_EXTHDR_OP_TCPOPT,
}

/// Interacts with layer 4 header options.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[nfnetlink_struct(nested = true)]
pub struct ExtHdr {
    #[field(NFTA_EXTHDR_DREG)]
    dreg: Register,
    #[field(NFTA_EXTHDR_TYPE)]
    typ: u8,
    #[field(NFTA_EXTHDR_OFFSET)]
    offset: u32,
    #[field(NFTA_EXTHDR_LEN)]
    len: u32,
    #[field(NFTA_EXTHDR_FLAGS)]
    flags: u32,
    #[field(NFTA_EXTHDR_OP)]
    op: ExtHdrOp,
    #[field(NFTA_EXTHDR_SREG)]
    sreg: Register,
}

impl Expression for ExtHdr {
    fn get_name() -> &'static str {
        "exthdr"
    }
}
