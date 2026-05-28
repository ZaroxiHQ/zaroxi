/*
Note: this file has been refactored. The original, full implementation of the
composition module has been moved into `crates/zaroxi-interface-desktop/src/desktop/composition/mod.rs`
and the composition module is now included here to preserve the original module
path (`crate::desktop::composition`) so that existing callers continue to work.

New files created under:
- crates/zaroxi-interface-desktop/src/desktop/composition/mod.rs
- crates/zaroxi-interface-desktop/src/desktop/composition/state.rs
- crates/zaroxi-interface-desktop/src/desktop/composition/refresh.rs
- crates/zaroxi-interface-desktop/src/desktop/composition/projections.rs
- crates/zaroxi-interface-desktop/src/desktop/composition/summary.rs

Behavior and public API are preserved. To run tests/harness:

  cargo test -p zaroxi-interface-desktop
  cargo test -p zaroxi-interface-app
  cargo run -p zaroxi-desktop-harness

*/
include!("composition/mod.rs");
