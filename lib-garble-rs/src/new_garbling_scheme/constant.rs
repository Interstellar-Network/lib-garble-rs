/// Kappa: κ in the paper
/// This is the "computational security parameter" which is for example 128 bits
pub(super) const KAPPA: usize = 128;

/// The relation between "l" and "l'" in the paper
/// defined as: l' = 8 * l
pub(super) const KAPPA_FACTOR: usize = 8;
