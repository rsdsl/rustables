use rustables_macros::{nfnetlink_enum, nfnetlink_struct};

use crate::sys::{
    NFTA_BYTEORDER_DREG, NFTA_BYTEORDER_LEN, NFTA_BYTEORDER_OP, NFTA_BYTEORDER_SIZE,
    NFTA_BYTEORDER_SREG, NFT_BYTEORDER_HTON, NFT_BYTEORDER_NTOH,
};

use super::{Expression, Register};

/// Byteorder operation.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[nfnetlink_enum(u32, nested = true)]
pub enum ByteorderOp {
    /// Network to host byte order.
    NtoH = NFT_BYTEORDER_NTOH,
    /// Host to network byte order.
    HtoN = NFT_BYTEORDER_HTON,
}

/// Ensures a register is of the correct byte order.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[nfnetlink_struct(nested = true)]
pub struct Byteorder {
    #[field(NFTA_BYTEORDER_SREG)]
    sreg: Register,
    #[field(NFTA_BYTEORDER_DREG)]
    dreg: Register,
    #[field(NFTA_BYTEORDER_OP)]
    op: ByteorderOp,
    #[field(NFTA_BYTEORDER_LEN)]
    len: u32,
    #[field(NFTA_BYTEORDER_SIZE)]
    siz: u32,
}

impl Expression for Byteorder {
    fn get_name() -> &'static str {
        "byteorder"
    }
}
