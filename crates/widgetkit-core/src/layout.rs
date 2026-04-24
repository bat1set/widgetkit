use crate::Size;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizePolicy {
    Fixed(Size),
    Content,
    ContentWithLimits {
        min: Option<Size>,
        max: Option<Size>,
    },
}

impl SizePolicy {
    pub const fn fixed(size: Size) -> Self {
        Self::Fixed(size)
    }

    pub const fn content() -> Self {
        Self::Content
    }

    pub const fn content_with_limits(min: Option<Size>, max: Option<Size>) -> Self {
        Self::ContentWithLimits { min, max }
    }

    pub fn constraints(self) -> Constraints {
        match self {
            Self::Fixed(size) => Constraints::new(Some(size), Some(size)),
            Self::Content => Constraints::unbounded(),
            Self::ContentWithLimits { min, max } => Constraints::new(min, max),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Constraints {
    pub min: Option<Size>,
    pub max: Option<Size>,
}

impl Constraints {
    pub const fn new(min: Option<Size>, max: Option<Size>) -> Self {
        Self { min, max }
    }

    pub const fn unbounded() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    pub fn clamp(self, size: Size) -> Size {
        let mut width = size.width.max(0.0);
        let mut height = size.height.max(0.0);

        if let Some(min) = self.min {
            width = width.max(min.width.max(0.0));
            height = height.max(min.height.max(0.0));
        }

        if let Some(max) = self.max {
            width = width.min(max.width.max(0.0));
            height = height.min(max.height.max(0.0));
        }

        Size::new(width, height)
    }
}
