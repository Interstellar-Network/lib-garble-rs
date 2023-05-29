use serde::{Deserialize, Serialize};

/// Represent a Wire's value, so essentially ON/OFF <=> a boolean
#[repr(transparent)]
#[derive(PartialEq, Debug, Serialize, Deserialize, Default, Clone)]
pub(crate) struct WireValue {
    pub(crate) value: bool,
}

impl PartialEq<bool> for WireValue {
    fn eq(&self, other: &bool) -> bool {
        &self.value == other
    }
}

impl PartialEq<bool> for &WireValue {
    fn eq(&self, other: &bool) -> bool {
        &self.value == other
    }
}

impl From<bool> for WireValue {
    fn from(value: bool) -> Self {
        Self { value }
    }
}

impl From<&u8> for WireValue {
    fn from(value: &u8) -> Self {
        Self { value: *value >= 1 }
    }
}
