use fancy_garbling::circuit::Circuit;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

// TODO!!! add the rest of skcd.proto
#[derive(Debug, Clone, Copy)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    // cf drawable::DigitSegmentsType
    // TODO!!! NOT PUB segments_type: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SkcdConfig {
    pub display_config: Option<DisplayConfig>,
}

/// Represents the raw(ie UNgarbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub(crate) struct InterstellarCircuit {
    pub(crate) circuit: Circuit,
    pub(crate) config: SkcdConfig,
}
