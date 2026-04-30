use crate::internal::WakeHandle;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use widgetkit_core::{Point, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowCommand {
    StartDrag,
    SetPosition(Point),
    SetSize(Size),
    SetVisible(bool),
    SetAlwaysOnTop(bool),
}

#[derive(Clone)]
pub struct WindowControl {
    sender: Sender<WindowCommand>,
    state: Arc<WindowState>,
    wake: WakeHandle,
}

impl WindowControl {
    pub(crate) fn new(
        sender: Sender<WindowCommand>,
        state: Arc<WindowState>,
        wake: WakeHandle,
    ) -> Self {
        Self {
            sender,
            state,
            wake,
        }
    }

    pub fn start_drag(&self) {
        self.send(WindowCommand::StartDrag);
    }

    pub fn set_position(&self, position: Point) {
        self.send(WindowCommand::SetPosition(position));
    }

    pub fn set_size(&self, size: Size) {
        if !size.is_empty() {
            self.send(WindowCommand::SetSize(size));
        }
    }

    pub fn hide(&self) {
        self.set_visible(false);
    }

    pub fn show(&self) {
        self.set_visible(true);
    }

    pub fn set_visible(&self, visible: bool) {
        self.state.set_visible(visible);
        self.send(WindowCommand::SetVisible(visible));
    }

    pub fn is_visible(&self) -> bool {
        self.state.is_visible()
    }

    pub fn set_always_on_top(&self, always_on_top: bool) {
        self.send(WindowCommand::SetAlwaysOnTop(always_on_top));
    }

    fn send(&self, command: WindowCommand) {
        if self.sender.send(command).is_ok() {
            self.wake.wake();
        }
    }
}

pub(crate) struct WindowCommandQueue {
    sender: Sender<WindowCommand>,
    receiver: Receiver<WindowCommand>,
    state: Arc<WindowState>,
}

impl WindowCommandQueue {
    pub(crate) fn new(visible: bool) -> Self {
        let (sender, receiver) = unbounded();
        Self {
            sender,
            receiver,
            state: Arc::new(WindowState::new(visible)),
        }
    }

    pub(crate) fn control(&self, wake: WakeHandle) -> WindowControl {
        WindowControl::new(self.sender.clone(), Arc::clone(&self.state), wake)
    }

    pub(crate) fn drain(&self) -> Vec<WindowCommand> {
        self.receiver.try_iter().collect()
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        self.state.set_visible(visible);
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.state.is_visible()
    }
}

pub(crate) struct WindowState {
    visible: AtomicBool,
}

impl WindowState {
    fn new(visible: bool) -> Self {
        Self {
            visible: AtomicBool::new(visible),
        }
    }

    fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::SeqCst);
    }

    fn is_visible(&self) -> bool {
        self.visible.load(Ordering::SeqCst)
    }
}
