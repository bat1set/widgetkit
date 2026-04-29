use crate::{Point, Size};

#[derive(Clone, Debug, PartialEq)]
pub enum HostEvent {
    CloseRequested,
    Resized(Size),
    ScaleFactorChanged(f32),
    Mouse(MouseEvent),
    Keyboard(KeyboardEvent),
    WindowFocused(bool),
    WindowVisible(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MouseEvent {
    Moved {
        position: Point,
    },
    Pressed {
        button: MouseButton,
        position: Point,
    },
    Released {
        button: MouseButton,
        position: Point,
    },
    Wheel {
        delta: MouseWheelDelta,
        position: Point,
    },
    Entered,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseWheelDelta {
    LineDelta { x: f32, y: f32 },
    PixelDelta { x: f32, y: f32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyboardEvent {
    Pressed { key: Key },
    Released { key: Key },
    TextInput(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Character(String),
    Named(String),
    Dead(Option<char>),
    Unidentified(String),
}
