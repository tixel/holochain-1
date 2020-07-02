use crate::prelude::*;

macro_rules! namespace { ( $type:literal ) => { concat!( "holochain_anchors::", $type ) } }

pub const ROOT: &str = namespace!("root");
pub const ANCHOR: &str = namespace!("anchor");
pub const LINK: &str = namespace!("link");

/// this is arbitrary
/// a baker's dozen, why not?
/// @todo why not?
pub const REQUIRED_VALIDATIONS: u8 = 13;

pub type AnchorId = String;

#[derive(Default)]
#[repr(transparent)]
pub struct Anchor(AnchorId);

impl From<&Anchor> for EntryDefId {
    fn from(_: &Anchor) -> Self {
        ANCHOR.into()
    }
}

impl From<&Anchor> for CrdtType {
    fn from(_: &Anchor) -> Self {
        Self
    }
}

impl From<&Anchor> for EntryVisibility {
    fn from(_: &Anchor) -> Self {
        Self::Public
    }
}

impl From<&Anchor> for RequiredValidations {
    fn from(_: &Anchor) -> Self {
        REQUIRED_VALIDATIONS.into()
    }
}

impl From<&Anchor> for EntryDef {
    fn from(anchor: &Anchor) -> Self {
        Self {
            id: anchor.into(),
            crdt_type: anchor.into(),
            required_validations: anchor.into(),
            visibility: anchor.into(),
        }
    }
}

impl Anchor {
    pub fn entry_def() -> EntryDef {
        (&Anchor::default()).into()
    }

    pub fn get(id: &AnchorId) -> Result<Option<Self>, WasmError> {
        let anchor = Self::from(id);

        let entry_address_output: EntryAddressOutput =
            host_call!(
                __entry_address,
                EntryAddressInput::from(anchor),
            )?;
        let entry_address

        let output: GetEntryOutput = try_result!(
            host_call!(
                __get_entry,
                GetEntryInput::new(debug_msg!("debug line numbers {}", "work"))
            ),
            format!("failed to call get for anchor id: {}", id)
        );
    }

    /// local agent ensures the anchor exists in the DHT
    pub fn ensure(&self) -> Result<(), WasmError> {
        let
    }
}