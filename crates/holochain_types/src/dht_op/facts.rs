//! Facts about DhtOps

use super::*;
use ::contrafact::*;
use crate::prelude::*;
use holo_hash::*;

fn _valid_dhtop() -> Facts<'static, DhtOp> {
    facts![prism(
        "DhtOp::Header::header_seq",
        |o: &mut DhtOp| o.header_seq_mut(),
        consecutive_int("chain", 0)
    ),]
}

impl DhtOp {
    /// Mutable access to the seq of the Header, if applicable
    pub fn header_seq_mut(&mut self) -> Option<&mut u32> {
        match self {
            DhtOp::StoreElement(_, ref mut h, _) => h.header_seq_mut(),
            DhtOp::StoreEntry(_, ref mut h, _) => Some(h.header_seq_mut()),
            DhtOp::RegisterAgentActivity(_, ref mut h) => h.header_seq_mut(),
            DhtOp::RegisterUpdatedContent(_, ref mut h, _) => Some(&mut h.header_seq),
            DhtOp::RegisterUpdatedElement(_, ref mut h, _) => Some(&mut h.header_seq),
            DhtOp::RegisterDeletedBy(_, ref mut h) => Some(&mut h.header_seq),
            DhtOp::RegisterDeletedEntryHeader(_, ref mut h) => Some(&mut h.header_seq),
            DhtOp::RegisterAddLink(_, ref mut h) => Some(&mut h.header_seq),
            DhtOp::RegisterRemoveLink(_, ref mut h) => Some(&mut h.header_seq),
        }
    }
}
