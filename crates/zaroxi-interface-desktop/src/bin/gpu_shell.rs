/*!
GPU shell compatibility stub.

The native GPU-backed binary previously compiled and ran a winit/pixels
event loop. Due to platform/API mismatches observed during CI and some
developer platforms, this file is now a small compatibility stub that
prints a helpful message and exits. The real GPU shell presenter remains
in the crate and is usable by unit tests; this stub avoids pulling the
native event loop into CI by default.

To run the full native GPU shell on a developer machine, enable the feature:
  cargo run -p zaroxi-interface-desktop --bin gpu_shell --features="gpu_shell_bin"
*/
fn main() {
    eprintln!("gpu_shell: native GPU shell is not started in this build.");
    eprintln!("If you intended to run the native windowed demo, enable the feature:");
    eprintln!("  cargo run -p zaroxi-interface-desktop --bin gpu_shell --features=\"gpu_shell_bin\"");
}
