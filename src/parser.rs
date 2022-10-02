use std::{
    collections::HashMap,
    fmt::Debug,
    mem::{self, size_of, transmute},
    str::Utf8Error,
    string::FromUtf8Error,
};

use libc::{
    nlattr, nlmsgerr, nlmsghdr, NFNETLINK_V0, NFNL_SUBSYS_NFTABLES, NLA_TYPE_MASK, NLMSG_MIN_TYPE,
    NLM_F_DUMP_INTR,
};
use thiserror::Error;

use crate::{
    nlmsg::{NfNetlinkObject, NfNetlinkWriter},
    InvalidProtocolFamily, ProtoFamily,
};

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("The buffer is too small to hold a valid message")]
    BufTooSmall,

    #[error("The message is too small")]
    NlMsgTooSmall,

    #[error("Invalid subsystem, expected NFTABLES")]
    InvalidSubsystem(u8),

    #[error("Invalid version, expected NFNETLINK_V0")]
    InvalidVersion(u8),

    #[error("Invalid port ID")]
    InvalidPortId(u32),

    #[error("Invalid sequence number")]
    InvalidSeq(u32),

    #[error("The generation number was bumped in the kernel while the operation was running, interrupting it")]
    ConcurrentGenerationUpdate,

    #[error("Unsupported message type")]
    UnsupportedType(u16),

    #[error("Invalid attribute type")]
    InvalidAttributeType,

    #[error("Unsupported attribute type")]
    UnsupportedAttributeType(u16),

    #[error("Unexpected message type")]
    UnexpectedType(u16),

    #[error("The decoded String is not UTF8 compliant")]
    StringDecodeFailure(#[from] FromUtf8Error),

    #[error("Invalid value for a protocol family")]
    InvalidProtocolFamily(#[from] InvalidProtocolFamily),

    #[error("A custom error occured")]
    Custom(Box<dyn std::error::Error + 'static>),
}

/// The largest nf_tables netlink message is the set element message, which contains the
/// NFTA_SET_ELEM_LIST_ELEMENTS attribute. This attribute is a nest that describes the set
/// elements. Given that the netlink attribute length (nla_len) is 16 bits, the largest message is
/// a bit larger than 64 KBytes.
pub fn nft_nlmsg_maxsize() -> u32 {
    u32::from(::std::u16::MAX) + unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u32
}

#[inline]
pub fn pad_netlink_object_with_variable_size(size: usize) -> usize {
    // align on a 4 bytes boundary
    (size + 3) & (!3)
}

#[inline]
pub fn pad_netlink_object<T>() -> usize {
    let size = size_of::<T>();
    // align on a 4 bytes boundary
    pad_netlink_object_with_variable_size(size)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Nfgenmsg {
    pub family: u8,  /* AF_xxx */
    pub version: u8, /* nfnetlink version */
    pub res_id: u16, /* resource id */
}

pub fn get_subsystem_from_nlmsghdr_type(x: u16) -> u8 {
    ((x & 0xff00) >> 8) as u8
}

pub fn get_operation_from_nlmsghdr_type(x: u16) -> u8 {
    (x & 0x00ff) as u8
}

pub fn get_nlmsghdr(buf: &[u8]) -> Result<nlmsghdr, DecodeError> {
    let size_of_hdr = size_of::<nlmsghdr>();

    if buf.len() < size_of_hdr {
        return Err(DecodeError::BufTooSmall);
    }

    let nlmsghdr_ptr = buf[0..size_of_hdr].as_ptr() as *const nlmsghdr;
    let nlmsghdr = unsafe { *nlmsghdr_ptr };

    if nlmsghdr.nlmsg_len as usize > buf.len() || (nlmsghdr.nlmsg_len as usize) < size_of_hdr {
        return Err(DecodeError::NlMsgTooSmall);
    }

    if nlmsghdr.nlmsg_flags & NLM_F_DUMP_INTR as u16 != 0 {
        return Err(DecodeError::ConcurrentGenerationUpdate);
    }

    Ok(nlmsghdr)
}
pub enum NlMsg<'a> {
    Done,
    Noop,
    Error(nlmsgerr),
    NfGenMsg(Nfgenmsg, &'a [u8]),
}

pub fn parse_nlmsg<'a>(buf: &'a [u8]) -> Result<(nlmsghdr, NlMsg<'a>), DecodeError> {
    // in theory the message is composed of the following parts:
    // - nlmsghdr (contains the message size and type)
    // - struct nlmsgerr OR nfgenmsg (nftables header that describes the message family)
    // - the raw value that we want to validate (if the previous part is nfgenmsg)
    let nlmsghdr = get_nlmsghdr(buf)?;

    let size_of_hdr = pad_netlink_object::<nlmsghdr>();

    if nlmsghdr.nlmsg_type < NLMSG_MIN_TYPE as u16 {
        match nlmsghdr.nlmsg_type as libc::c_int {
            NLMSG_NOOP => return Ok((nlmsghdr, NlMsg::Noop)),
            NLMSG_ERROR => {
                if nlmsghdr.nlmsg_len as usize > buf.len()
                    || (nlmsghdr.nlmsg_len as usize) < size_of_hdr + size_of::<nlmsgerr>()
                {
                    return Err(DecodeError::NlMsgTooSmall);
                }
                let mut err = unsafe {
                    *(buf[size_of_hdr..size_of_hdr + size_of::<nlmsgerr>()].as_ptr()
                        as *const nlmsgerr)
                };
                // some APIs return negative values, while other return positive values
                err.error = err.error.abs();
                return Ok((nlmsghdr, NlMsg::Error(err)));
            }
            NLMSG_DONE => return Ok((nlmsghdr, NlMsg::Done)),
            x => return Err(DecodeError::UnsupportedType(x as u16)),
        }
    }

    let subsys = get_subsystem_from_nlmsghdr_type(nlmsghdr.nlmsg_type);
    if subsys != NFNL_SUBSYS_NFTABLES as u8 {
        return Err(DecodeError::InvalidSubsystem(subsys));
    }

    let size_of_nfgenmsg = pad_netlink_object::<Nfgenmsg>();
    if nlmsghdr.nlmsg_len as usize > buf.len()
        || (nlmsghdr.nlmsg_len as usize) < size_of_hdr + size_of_nfgenmsg
    {
        return Err(DecodeError::NlMsgTooSmall);
    }

    let nfgenmsg_ptr = buf[size_of_hdr..size_of_hdr + size_of_nfgenmsg].as_ptr() as *const Nfgenmsg;
    let nfgenmsg = unsafe { *nfgenmsg_ptr };
    let subsys = get_subsystem_from_nlmsghdr_type(nlmsghdr.nlmsg_type);
    if subsys != NFNL_SUBSYS_NFTABLES as u8 {
        return Err(DecodeError::InvalidSubsystem(subsys));
    }
    if nfgenmsg.version != NFNETLINK_V0 as u8 {
        return Err(DecodeError::InvalidVersion(nfgenmsg.version));
    }

    let raw_value = &buf[size_of_hdr + size_of_nfgenmsg..nlmsghdr.nlmsg_len as usize];

    Ok((nlmsghdr, NlMsg::NfGenMsg(nfgenmsg, raw_value)))
}

pub type NetlinkType = u16;

pub trait NfNetlinkAttribute: Debug + Sized {
    fn get_size(&self) -> usize {
        size_of::<Self>()
    }

    unsafe fn write_payload(&self, addr: *mut u8);
    // example body: std::ptr::copy_nonoverlapping(self as *const Self as *const u8, addr, self.get_size());
}

/// Write the attribute, preceded by a `libc::nlattr`
// rewrite of `mnl_attr_put`
fn write_attribute<'a>(ty: NetlinkType, obj: &Attribute, writer: &mut NfNetlinkWriter<'a>) {
    // copy the header
    let header_len = pad_netlink_object::<libc::nlattr>();
    let header = libc::nlattr {
        // nla_len contains the header size + the unpadded attribute length
        nla_len: (header_len + obj.get_size() as usize) as u16,
        nla_type: ty,
    };

    let buf = writer.add_data_zeroed(header_len);
    unsafe {
        std::ptr::copy_nonoverlapping(
            &header as *const libc::nlattr as *const u8,
            buf.as_mut_ptr(),
            header_len as usize,
        );
    }

    let buf = writer.add_data_zeroed(obj.get_size());
    // copy the attribute data itself
    unsafe {
        obj.write_payload(buf.as_mut_ptr());
    }
}

impl NfNetlinkAttribute for ProtoFamily {
    unsafe fn write_payload(&self, addr: *mut u8) {
        *(addr as *mut u32) = *self as u32;
    }
}

impl NfNetlinkAttribute for u8 {
    unsafe fn write_payload(&self, addr: *mut u8) {
        *addr = *self;
    }
}

impl NfNetlinkAttribute for u16 {
    unsafe fn write_payload(&self, addr: *mut u8) {
        *(addr as *mut Self) = *self;
    }
}

impl NfNetlinkAttribute for u32 {
    unsafe fn write_payload(&self, addr: *mut u8) {
        *(addr as *mut Self) = *self;
    }
}

impl NfNetlinkAttribute for u64 {
    unsafe fn write_payload(&self, addr: *mut u8) {
        *(addr as *mut Self) = *self;
    }
}

impl NfNetlinkAttribute for String {
    fn get_size(&self) -> usize {
        self.len()
    }

    unsafe fn write_payload(&self, addr: *mut u8) {
        std::ptr::copy_nonoverlapping(self.as_bytes().as_ptr(), addr, self.len());
    }
}

impl NfNetlinkAttribute for Vec<u8> {
    fn get_size(&self) -> usize {
        self.len()
    }

    unsafe fn write_payload(&self, addr: *mut u8) {
        std::ptr::copy_nonoverlapping(self.as_ptr(), addr, self.len());
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NfNetlinkAttributes {
    attributes: HashMap<NetlinkType, Attribute>,
}

impl NfNetlinkAttributes {
    pub fn new() -> Self {
        NfNetlinkAttributes {
            attributes: HashMap::new(),
        }
    }

    pub fn set_attr(&mut self, ty: NetlinkType, obj: Attribute) {
        self.attributes.insert(ty, obj);
    }

    pub fn get_attr(&self, ty: NetlinkType) -> Option<&Attribute> {
        self.attributes.get(&ty)
    }
}

pub struct NfNetlinkAttributeReader<'a> {
    buf: &'a [u8],
    pos: usize,
    remaining_size: usize,
    attrs: NfNetlinkAttributes,
}

impl<'a> NfNetlinkAttributeReader<'a> {
    pub fn new(buf: &'a [u8], remaining_size: usize) -> Result<Self, DecodeError> {
        if buf.len() < remaining_size {
            return Err(DecodeError::BufTooSmall);
        }

        Ok(Self {
            buf,
            pos: 0,
            remaining_size,
            attrs: NfNetlinkAttributes::new(),
        })
    }

    pub fn get_raw_data(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }

    pub fn decode<T: NfNetlinkObject>(mut self) -> Result<NfNetlinkAttributes, DecodeError> {
        while self.remaining_size > pad_netlink_object::<nlattr>() {
            let nlattr =
                unsafe { *transmute::<*const u8, *const nlattr>(self.buf[self.pos..].as_ptr()) };
            // TODO: ignore the byteorder and nested attributes for now
            let nla_type = nlattr.nla_type & NLA_TYPE_MASK as u16;

            self.pos += pad_netlink_object::<nlattr>();
            let attr_remaining_size = nlattr.nla_len as usize - pad_netlink_object::<nlattr>();
            self.attrs.set_attr(
                nla_type,
                T::decode_attribute(
                    nla_type,
                    &self.buf[self.pos..self.pos + attr_remaining_size],
                )?,
            );
            self.pos += pad_netlink_object_with_variable_size(attr_remaining_size);

            self.remaining_size -= pad_netlink_object_with_variable_size(nlattr.nla_len as usize);
        }

        Ok(self.attrs)
    }
}

pub fn parse_object<'a>(
    hdr: nlmsghdr,
    msg: NlMsg<'a>,
    buf: &'a [u8],
) -> Result<(Nfgenmsg, NfNetlinkAttributeReader<'a>, &'a [u8]), DecodeError> {
    let remaining_size = hdr.nlmsg_len as usize
        - pad_netlink_object_with_variable_size(size_of::<nlmsghdr>() + size_of::<Nfgenmsg>());

    let remaining_data = &buf[pad_netlink_object_with_variable_size(hdr.nlmsg_len as usize)..];

    match msg {
        NlMsg::NfGenMsg(nfgenmsg, content) => Ok((
            nfgenmsg,
            NfNetlinkAttributeReader::new(content, remaining_size)?,
            remaining_data,
        )),
        _ => Err(DecodeError::UnexpectedType(hdr.nlmsg_type)),
    }
}

pub trait SerializeNfNetlink {
    fn serialize<'a>(&self, writer: &mut NfNetlinkWriter<'a>);
}

impl SerializeNfNetlink for NfNetlinkAttributes {
    fn serialize<'a>(&self, writer: &mut NfNetlinkWriter<'a>) {
        // TODO: improve performance by not sorting this
        let mut keys: Vec<&NetlinkType> = self.attributes.keys().collect();
        keys.sort();
        for k in keys {
            write_attribute(*k, self.attributes.get(k).unwrap(), writer);
        }
    }
}

macro_rules! impl_attribute {
    ($enum_name:ident, $([$internal_name:ident, $type:ty]),+) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $enum_name {
            $(
                $internal_name($type),
            )+
        }

        impl NfNetlinkAttribute for $enum_name {
            fn get_size(&self) -> usize {
                 match self {
                    $(
                        $enum_name::$internal_name(val) => val.get_size()
                    ),+
                 }
            }

            unsafe fn write_payload(&self, addr: *mut u8) {
                match self {
                    $(
                        $enum_name::$internal_name(val) => val.write_payload(addr)
                    ),+
                 }

            }
        }

        impl $enum_name {
            $(
                #[allow(non_snake_case)]
                pub fn $internal_name(&self) -> Option<&$type> {
                    match self {
                        $enum_name::$internal_name(val) => Some(val),
                        _ => None
                    }
                }
            )+
        }
    };
}

impl_attribute!(
    Attribute,
    [String, String],
    [U8, u8],
    [U16, u16],
    [U32, u32],
    [U64, u64],
    [VecU8, Vec<u8>],
    [ProtoFamily, ProtoFamily]
);

#[macro_export]
macro_rules! impl_attr_getters_and_setters {
    ($struct:ident, [$(($getter_name:ident, $setter_name:ident, $attr_name:expr, $internal_name:ident, $type:ty)),+]) => {
        impl $struct {
            $(
                #[allow(dead_code)]
                pub fn $getter_name(&self) -> Option<&$type> {
                    self.inner.get_attr($attr_name as $crate::parser::NetlinkType).map(|x| x.$internal_name()).flatten()
                }

                #[allow(dead_code)]
                pub fn $setter_name(&mut self, val: $type) {
                    self.inner.set_attr($attr_name as $crate::parser::NetlinkType, $crate::parser::Attribute::$internal_name(val));
                }
            )+
        }
    };
}
