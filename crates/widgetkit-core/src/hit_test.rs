#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTest {
    Miss,
    Hit,
    Draggable,
    Resize(ResizeEdge),
    Transparent,
}

impl HitTest {
    pub const fn hit_if(hit: bool) -> Self {
        if hit { Self::Hit } else { Self::Miss }
    }

    pub const fn accepts_input(self) -> bool {
        matches!(self, Self::Hit | Self::Draggable | Self::Resize(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
