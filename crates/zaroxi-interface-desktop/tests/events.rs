use zaroxi_interface_desktop::events::{
    FrameModel, Key, RenderViewModel, Region, UiEvent, EventRouter,
};

struct MockFrameModel {
    focused_line: u32,
    active_buffer: Option<String>,
}

impl MockFrameModel {
    fn new() -> Self {
        Self {
            focused_line: 0,
            active_buffer: None,
        }
    }
}

impl FrameModel for MockFrameModel {
    fn focused_line(&self) -> u32 {
        self.focused_line
    }

    fn move_focus_down(&mut self) {
        self.focused_line = self.focused_line.saturating_add(1);
    }

    fn activate_current_buffer(&mut self) {
        // for the mock, activating sets a simple placeholder name
        self.active_buffer = Some(format!("buffer_at_line_{}", self.focused_line));
    }

    fn set_active_buffer(&mut self, name: String) {
        self.active_buffer = Some(name);
    }
}

struct MockRenderViewModel {
    active_section: Region,
}

impl MockRenderViewModel {
    fn new() -> Self {
        Self {
            active_section: Region::Content,
        }
    }
}

impl RenderViewModel for MockRenderViewModel {
    fn set_active_section(&mut self, region: Region) {
        self.active_section = region;
    }

    fn active_section(&self) -> Region {
        self.active_section.clone()
    }
}

#[test]
fn arrow_down_moves_focus_in_content_region() {
    let mut router = EventRouter::new();
    router.set_active_region(Region::Content);

    let mut frame = MockFrameModel::new();
    let mut view = MockRenderViewModel::new();

    assert_eq!(frame.focused_line(), 0);

    router.handle_ui_event(UiEvent::Key(Key::ArrowDown), &mut frame, &mut view);

    assert_eq!(frame.focused_line(), 1);
}

#[test]
fn enter_activates_buffer() {
    let mut router = EventRouter::new();
    router.set_active_region(Region::Content);

    let mut frame = MockFrameModel::new();
    let mut view = MockRenderViewModel::new();

    // move focus down twice then activate
    router.handle_ui_event(UiEvent::Key(Key::ArrowDown), &mut frame, &mut view);
    router.handle_ui_event(UiEvent::Key(Key::ArrowDown), &mut frame, &mut view);

    assert_eq!(frame.focused_line(), 2);

    router.handle_ui_event(UiEvent::Key(Key::Enter), &mut frame, &mut view);

    assert_eq!(
        frame.active_buffer,
        Some("buffer_at_line_2".to_string())
    );
}
