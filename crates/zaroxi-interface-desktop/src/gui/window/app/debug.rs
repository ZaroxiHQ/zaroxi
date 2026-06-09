/*!
Shared debug/trace helpers used across the app module and its children.
*/

pub(crate) fn gui_debug(msg: &str) {
    if std::env::var("ZAROXI_DEBUG_GUI").as_deref() == Ok("1") {
        eprintln!("{}", msg);
    }
}

pub(crate) fn event_label(event: &winit::event::WindowEvent) -> String {
    use winit::event::WindowEvent;
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            format!("CursorMoved({:.0},{:.0})", position.x, position.y)
        }
        WindowEvent::MouseInput { state, button, .. } => {
            format!("MouseInput({:?},{:?})", state, button)
        }
        WindowEvent::MouseWheel { .. } => "MouseWheel".into(),
        WindowEvent::RedrawRequested => "RedrawRequested".into(),
        WindowEvent::Resized(s) => format!("Resized({}x{})", s.width, s.height),
        WindowEvent::ScaleFactorChanged { .. } => "ScaleFactorChanged".into(),
        WindowEvent::CursorEntered { .. } => "CursorEntered".into(),
        WindowEvent::CursorLeft { .. } => "CursorLeft".into(),
        WindowEvent::Focused(f) => format!("Focused({})", f),
        WindowEvent::CloseRequested => "CloseRequested".into(),
        WindowEvent::ModifiersChanged(_) => "ModifiersChanged".into(),
        WindowEvent::Occluded(b) => format!("Occluded({})", b),
        WindowEvent::ThemeChanged(_) => "ThemeChanged".into(),
        WindowEvent::Touch(_) => "Touch".into(),
        WindowEvent::PinchGesture { .. } => "PinchGesture".into(),
        other => format!("other({})", variant_name(other)),
    }
}

fn variant_name(_ev: &winit::event::WindowEvent) -> &'static str {
    "unknown"
}

pub(crate) fn click_trace(msg: &str) {
    if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
        eprintln!("{}", msg);
    }
}

macro_rules! click_trace_fmt {
    ($($arg:tt)*) => {
        if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
            eprintln!($($arg)*);
        }
    };
}
pub(crate) use click_trace_fmt;

macro_rules! gui_debug_fmt {
    ($($arg:tt)*) => {
        if std::env::var("ZAROXI_DEBUG_GUI").as_deref() == Ok("1") {
            eprintln!($($arg)*);
        }
    };
}
pub(crate) use gui_debug_fmt;
