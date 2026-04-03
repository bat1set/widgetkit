use crate::Size;

#[derive(Clone, Debug, PartialEq)]
pub enum HostEvent {
    CloseRequested,
    Focused(bool),
    Resized(Size),
}
