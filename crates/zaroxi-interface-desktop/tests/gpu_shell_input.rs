use zaroxi_interface_desktop::gpu_shell_adapter::{NativeKey, map_native_to_ui_event};
use zaroxi_interface_desktop::events::{UiEvent, Key as UiKey};

#[test]
fn native_key_maps_to_ui_event() {
    assert_eq!(map_native_to_ui_event(NativeKey::Up), Some(UiEvent::Key(UiKey::ArrowUp)));
    assert_eq!(map_native_to_ui_event(NativeKey::Down), Some(UiEvent::Key(UiKey::ArrowDown)));
    assert_eq!(map_native_to_ui_event(NativeKey::Enter), Some(UiEvent::Key(UiKey::Enter)));
    assert_eq!(map_native_to_ui_event(NativeKey::Char('x')), Some(UiEvent::Key(UiKey::Char('x'))));
}
