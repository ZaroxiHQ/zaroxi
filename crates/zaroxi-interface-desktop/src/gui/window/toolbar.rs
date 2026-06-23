/*!
Top toolbar / titlebar panel.

Phase 50: panel-owned UiBlock construction.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

pub struct TopBarPanel;

impl TopBarPanel {
    pub fn build_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        UiBlock {
            id: r.id.to_string(),
            title: "Zaroxi Studio".to_string(),
            rect: r.into(),
            header_color: Some(tokens.titlebar_background.to_array()),
            header_only: true,
            text_color: Some(tokens.text_primary.to_array()),
            ..Default::default()
        }
    }
}
