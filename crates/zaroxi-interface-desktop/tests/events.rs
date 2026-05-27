use zaroxi_interface_desktop::events::{
    Action, ActionExecutor, EventBridge, EventRouter, FrameModel, Key, Region, RenderViewModel,
    UiEvent,
};

struct MockFrameModel {
    focused_line: u32,
    active_buffer: Option<String>,
}

impl MockFrameModel {
    fn new() -> Self {
        Self { focused_line: 0, active_buffer: None }
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
        Self { active_section: Region::Content }
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

    assert_eq!(frame.active_buffer, Some("buffer_at_line_2".to_string()));
}

struct MockActionExecutor {
    last_action: Option<Action>,
}

impl MockActionExecutor {
    fn new() -> Self {
        Self { last_action: None }
    }
}

impl ActionExecutor for MockActionExecutor {
    fn execute(&mut self, action: Action) {
        self.last_action = Some(action);
    }
}

#[test]
fn bridge_maps_enter_to_insert_newline_action() {
    let mut exec = MockActionExecutor::new();

    EventBridge::handle_event(UiEvent::Key(Key::Enter), &mut exec);

    assert_eq!(exec.last_action, Some(Action::InsertNewLine));
}

#[test]
fn bridge_maps_char_to_set_active_buffer_action() {
    let mut exec = MockActionExecutor::new();

    EventBridge::handle_event(UiEvent::Key(Key::Char('x')), &mut exec);

    assert_eq!(exec.last_action, Some(Action::SetActiveBuffer("x".to_string())));
}
