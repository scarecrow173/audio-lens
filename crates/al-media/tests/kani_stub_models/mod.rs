//! Test-only Kani stub models for the al#274 solver-floor spike.
//!
//! These replacements are compiled only under `cfg(kani)` and are never
//! part of the proptest or product code paths. Each harness that uses one
//! must document why the abstracted behavior is outside the property under
//! proof.

/// Replace diagnostic formatting with an empty string. Valid only for
/// proofs whose contract never inspects human-readable error text nor any
/// `format!`-produced data value.
pub(crate) fn fmt_format_empty(_args: std::fmt::Arguments<'_>) -> String {
    String::new()
}

/// Make allocator-failure paths infeasible at the failure boundary. Valid
/// only while the lane uses `--no-malloc-may-fail`; capacity overflow and
/// layout arithmetic remain outside this stub and are still verified.
pub(crate) fn handle_alloc_error_infallible(_layout: std::alloc::Layout) -> ! {
    kani::assume(false);
    loop {}
}
