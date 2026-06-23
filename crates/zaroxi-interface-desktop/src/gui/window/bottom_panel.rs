/*!
Bottom dock panel.

Phase 50: panel-owned UiBlock construction.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

pub struct BottomDockPanel;

impl BottomDockPanel {
    pub fn build_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        UiBlock {
            id: r.id.to_string(),
            visible: r.rect.height > 0,
            rect: r.into(),
            header_color: Some(tokens.app_background.to_array()),
            header_only: true,
            ..Default::default()
        }
    }
}
