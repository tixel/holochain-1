use super::*;

impl NewEntryHeader {
    pub fn header_seq_mut(&mut self) -> &mut u32 {
        match self {
            Self::Create(Create {
                ref mut header_seq, ..
            }) => header_seq,
            Self::Update(Update {
                ref mut header_seq, ..
            }) => header_seq,
        }
    }
}
