//! This module previously held a `SyntaxDocument` wrapper that tied
//! document text to a syntax tree.  That wrapper is **no longer used**
//! by the current architecture, which relies on the plain‑text
//! `Document` from `zaroxi_domain_editor`.  This file remains for
//! backward compatibility and will be removed in a future cleanup.
//!
//! Please use `zaroxi_domain_editor::Document` for text operations
//! and the modules in this crate (`language`, `parser`, `highlight`,
//! `cache`) for syntax‑aware features.
