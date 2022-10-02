//! Adds a table, two chains and some rules to netfilter.
//!
//! This example uses `verdict accept` everywhere. So even after running this the firewall won't
//! block anything. This is so anyone trying to run this does not end up in a strange state
//! where they don't understand why their network is broken. Try changing to `verdict drop` if
//! you want to see the block working.
//!
//! Run the following to print out current active tables, chains and rules in netfilter. Must be
//! executed as root:
//! ```bash
//! # nft list ruleset
//! ```
//! After running this example, the output should be the following:
//! ```ignore
//! table inet example-table {
//!         chain chain-for-outgoing-packets {
//!                 type filter hook output priority 0; policy accept;
//!                 ip daddr 10.1.0.0/24 counter packets 0 bytes 0 accept
//!         }
//!
//!         chain chain-for-incoming-packets {
//!                 type filter hook input priority 0; policy accept;
//!                 iif "lo" accept
//!         }
//! }
//! ```
//!
//! Try pinging any IP in the network range denoted by the outgoing rule and see the counter
//! increment:
//! ```bash
//! $ ping 10.1.0.7
//! ```
//!
//! Everything created by this example can be removed by running
//! ```bash
//! # nft delete table inet example-table
//! ```

use ipnetwork::{IpNetwork, Ipv4Network};
//use rustables::{nft_expr, query::send_batch, sys::libc, Batch, Chain, ProtoFamily, Rule, Table};
use std::{ffi::CString, io, net::Ipv4Addr, rc::Rc};
//
//const TABLE_NAME: &str = "example-table";
//const OUT_CHAIN_NAME: &str = "chain-for-outgoing-packets";
//const IN_CHAIN_NAME: &str = "chain-for-incoming-packets";

fn main() -> Result<(), Error> {
    //    // Create a batch. This is used to store all the netlink messages we will later send.
    //    // Creating a new batch also automatically writes the initial batch begin message needed
    //    // to tell netlink this is a single transaction that might arrive over multiple netlink packets.
    //    let mut batch = Batch::new();
    //
    //    // Create a netfilter table operating on both IPv4 and IPv6 (ProtoFamily::Inet)
    //    let table = Rc::new(Table::new(TABLE_NAME, ProtoFamily::Inet));
    //    // Add the table to the batch with the `MsgType::Add` type, thus instructing netfilter to add
    //    // this table under its `ProtoFamily::Inet` ruleset.
    //    batch.add(&Rc::clone(&table), rustables::MsgType::Add);
    //
    //    // Create input and output chains under the table we created above.
    //    // Hook the chains to the input and output event hooks, with highest priority (priority zero).
    //    // See the `Chain::set_hook` documentation for details.
    //    let mut out_chain = Chain::new(OUT_CHAIN_NAME, Rc::clone(&table));
    //    let mut in_chain = Chain::new(IN_CHAIN_NAME, Rc::clone(&table));
    //
    //    out_chain.set_hook(rustables::Hook::Out, 0);
    //    in_chain.set_hook(rustables::Hook::In, 0);
    //
    //    // Set the default policies on the chains. If no rule matches a packet processed by the
    //    // `out_chain` or the `in_chain` it will accept the packet.
    //    out_chain.set_policy(rustables::Policy::Accept);
    //    in_chain.set_policy(rustables::Policy::Accept);
    //
    //    let out_chain = Rc::new(out_chain);
    //    let in_chain = Rc::new(in_chain);
    //
    //    // Add the two chains to the batch with the `MsgType` to tell netfilter to create the chains
    //    // under the table.
    //    batch.add(&Rc::clone(&out_chain), rustables::MsgType::Add);
    //    batch.add(&Rc::clone(&in_chain), rustables::MsgType::Add);
    //
    //    // === ADD RULE ALLOWING ALL TRAFFIC TO THE LOOPBACK DEVICE ===
    //
    //    // Create a new rule object under the input chain.
    //    let mut allow_loopback_in_rule = Rule::new(Rc::clone(&in_chain));
    //    // Lookup the interface index of the loopback interface.
    //    let lo_iface_index = iface_index("lo")?;
    //
    //    // First expression to be evaluated in this rule is load the meta information "iif"
    //    // (incoming interface index) into the comparison register of netfilter.
    //    // When an incoming network packet is processed by this rule it will first be processed by this
    //    // expression, which will load the interface index of the interface the packet came from into
    //    // a special "register" in netfilter.
    //    allow_loopback_in_rule.add_expr(&nft_expr!(meta iif));
    //    // Next expression in the rule is to compare the value loaded into the register with our desired
    //    // interface index, and succeed only if it's equal. For any packet processed where the equality
    //    // does not hold the packet is said to not match this rule, and the packet moves on to be
    //    // processed by the next rule in the chain instead.
    //    allow_loopback_in_rule.add_expr(&nft_expr!(cmp == lo_iface_index));
    //
    //    // Add a verdict expression to the rule. Any packet getting this far in the expression
    //    // processing without failing any expression will be given the verdict added here.
    //    allow_loopback_in_rule.add_expr(&nft_expr!(verdict accept));
    //
    //    // Add the rule to the batch.
    //    batch.add(&allow_loopback_in_rule, rustables::MsgType::Add);
    //
    //    // === ADD A RULE ALLOWING (AND COUNTING) ALL PACKETS TO THE 10.1.0.0/24 NETWORK ===
    //
    //    let mut block_out_to_private_net_rule = Rule::new(Rc::clone(&out_chain));
    //    let private_net_ip = Ipv4Addr::new(10, 1, 0, 0);
    //    let private_net_prefix = 24;
    //    let private_net = IpNetwork::V4(Ipv4Network::new(private_net_ip, private_net_prefix)?);
    //
    //    // Load the `nfproto` metadata into the netfilter register. This metadata denotes which layer3
    //    // protocol the packet being processed is using.
    //    block_out_to_private_net_rule.add_expr(&nft_expr!(meta nfproto));
    //    // Check if the currently processed packet is an IPv4 packet. This must be done before payload
    //    // data assuming the packet uses IPv4 can be loaded in the next expression.
    //    block_out_to_private_net_rule.add_expr(&nft_expr!(cmp == libc::NFPROTO_IPV4 as u8));
    //
    //    // Load the IPv4 destination address into the netfilter register.
    //    block_out_to_private_net_rule.add_expr(&nft_expr!(payload ipv4 daddr));
    //    // Mask out the part of the destination address that is not part of the network bits. The result
    //    // of this bitwise masking is stored back into the same netfilter register.
    //    block_out_to_private_net_rule.add_expr(&nft_expr!(bitwise mask private_net.mask(), xor 0));
    //    // Compare the result of the masking with the IP of the network we are interested in.
    //    block_out_to_private_net_rule.add_expr(&nft_expr!(cmp == private_net.ip()));
    //
    //    // Add a packet counter to the rule. Shows how many packets have been evaluated against this
    //    // expression. Since expressions are evaluated from first to last, putting this counter before
    //    // the above IP net check would make the counter increment on all packets also *not* matching
    //    // those expressions. Because the counter would then be evaluated before it fails a check.
    //    // Similarly, if the counter was added after the verdict it would always remain at zero. Since
    //    // when the packet hits the verdict expression any further processing of expressions stop.
    //    block_out_to_private_net_rule.add_expr(&nft_expr!(counter));
    //
    //    // Accept all the packets matching the rule so far.
    //    block_out_to_private_net_rule.add_expr(&nft_expr!(verdict accept));
    //
    //    // Add the rule to the batch. Without this nothing would be sent over netlink and netfilter,
    //    // and all the work on `block_out_to_private_net_rule` so far would go to waste.
    //    batch.add(&block_out_to_private_net_rule, rustables::MsgType::Add);
    //
    //    // === ADD A RULE ALLOWING ALL OUTGOING ICMPv6 PACKETS WITH TYPE 133 AND CODE 0 ===
    //
    //    let mut allow_router_solicitation = Rule::new(Rc::clone(&out_chain));
    //
    //    // Check that the packet is IPv6 and ICMPv6
    //    allow_router_solicitation.add_expr(&nft_expr!(meta nfproto));
    //    allow_router_solicitation.add_expr(&nft_expr!(cmp == libc::NFPROTO_IPV6 as u8));
    //    allow_router_solicitation.add_expr(&nft_expr!(meta l4proto));
    //    allow_router_solicitation.add_expr(&nft_expr!(cmp == libc::IPPROTO_ICMPV6 as u8));
    //
    //    allow_router_solicitation.add_expr(&rustables::expr::Payload::Transport(
    //        rustables::expr::TransportHeaderField::Icmpv6(rustables::expr::Icmpv6HeaderField::Type),
    //    ));
    //    allow_router_solicitation.add_expr(&nft_expr!(cmp == 133u8));
    //    allow_router_solicitation.add_expr(&rustables::expr::Payload::Transport(
    //        rustables::expr::TransportHeaderField::Icmpv6(rustables::expr::Icmpv6HeaderField::Code),
    //    ));
    //    allow_router_solicitation.add_expr(&nft_expr!(cmp == 0u8));
    //
    //    allow_router_solicitation.add_expr(&nft_expr!(verdict accept));
    //
    //    batch.add(&allow_router_solicitation, rustables::MsgType::Add);
    //
    //    // === FINALIZE THE TRANSACTION AND SEND THE DATA TO NETFILTER ===
    //
    //    // Finalize the batch. This means the batch end message is written into the batch, telling
    //    // netfilter the we reached the end of the transaction message. It's also converted to a type
    //    // that implements `IntoIterator<Item = &'a [u8]>`, thus allowing us to get the raw netlink data
    //    // out so it can be sent over a netlink socket to netfilter.
    //    match batch.finalize() {
    //        Some(mut finalized_batch) => {
    //            // Send the entire batch and process any returned messages.
    //            send_batch(&mut finalized_batch)?;
    //            Ok(())
    //        }
    //        None => todo!(),
    //    }
    Ok(())
}

// Look up the interface index for a given interface name.
fn iface_index(name: &str) -> Result<libc::c_uint, Error> {
    let c_name = CString::new(name)?;
    let index = unsafe { libc::if_nametoindex(c_name.as_ptr()) };
    if index == 0 {
        Err(Error::from(io::Error::last_os_error()))
    } else {
        Ok(index)
    }
}

#[derive(Debug)]
struct Error(String);

impl<T: std::error::Error> From<T> for Error {
    fn from(error: T) -> Self {
        Error(error.to_string())
    }
}
