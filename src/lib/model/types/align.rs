//! Module defining the alignment enums.

#![allow(missing_docs)]  // Because IterVariants! produces undocumented methods.


macro_attr! {
    /// Horizontal alignment of text within a rectangle.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
             Deserialize, IterVariants!(HAligns))]
    #[serde(rename_all = "lowercase")]
    pub enum HAlign {
        /// Left alignment.
        Left,
        /// Horizontal centering.
        Center,
        /// Right alignment.
        Right,
    }
}

macro_attr! {
    /// Vertical alignment of text within a rectangle.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
             Deserialize, IterVariants!(VAligns))]
    #[serde(rename_all = "lowercase")]
    pub enum VAlign {
        /// Top alignment.
        Top,
        /// Vertical centering.
        Middle,
        /// Bottom alignment.
        Bottom,
    }
}
