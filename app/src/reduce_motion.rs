//! The system's Reduce Motion setting (Accessibility › Display), for the sidebar icons' hover
//! motion (DESIGN §13, P5-26): with it on they keep their colours and grounds but do not move.
use objc2_app_kit::NSWorkspace;

/// Whether the system asks for reduced motion now. Cheap enough to read on every hover, so a
/// change in System Settings shows the next time the pointer comes in.
pub fn on() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}
