use rustables_macros::{nfnetlink_enum, nfnetlink_struct};

use crate::sys::{
    NFTA_RT_DREG, NFTA_RT_KEY, NFT_RT_CLASSID, NFT_RT_NEXTHOP4, NFT_RT_NEXTHOP6, NFT_RT_TCPMSS,
};

use super::{Expression, Register};

/// Kind of routing information.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[nfnetlink_enum(u32, nested = true)]
pub enum RtKey {
    /// Class ID.
    ClassID = NFT_RT_CLASSID,
    /// Next IPv4 hop.
    NextHop4 = NFT_RT_NEXTHOP4,
    /// Next IPv6 hop.
    NextHop6 = NFT_RT_NEXTHOP6,
    /// TCP MSS.
    TCPMSS = NFT_RT_TCPMSS,
}

/// Loads routing information into a register.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[nfnetlink_struct(nested = true)]
pub struct Rt {
    #[field(NFTA_RT_DREG)]
    dreg: Register,
    #[field(NFTA_RT_KEY)]
    key: RtKey,
}

impl Expression for Rt {
    fn get_name() -> &'static str {
        "rt"
    }
}
