pub enum Event<M> {
    Message(M),
    Host(widgetkit_core::HostEvent),
}
