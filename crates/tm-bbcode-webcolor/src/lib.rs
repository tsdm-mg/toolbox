use racros::AutoStr;

/// Web colors supported in bbcode.
///
/// There are 40 kinds of pre-defined colors available.
///
/// And a custom color [`WebColors::Custom`] which represent its value in string.
#[derive(AutoStr)]
#[autorule = "PascalCase"]
pub enum WebColor {
    Black,
    Sienna,
    DarkOliveGreen,
    DarkGreen,
    DarkSlateBlue,
    Navy,
    Indigo,
    DarkSlateGray,
    DarkRed,
    DarkOrange,
    Olive,
    Green,
    Teal,
    Blue,
    SlateGray,
    DimGray,
    Red,
    SandyBrown,
    YellowGreen,
    SeaGreen,
    MediumTurquoise,
    RoyalBlue,
    Purple,
    Gray,
    Magenta,
    Orange,
    Yellow,
    Lime,
    Cyan,
    DeepSkyBlue,
    DarkOrchid,
    Silver,
    Pink,
    Wheat,
    LemonChiffon,
    PaleGreen,
    PaleTurquoise,
    LightBlue,
    Plum,
    White,
    /// Custom web color value.
    ///
    /// Supported format shall be the ones supported in css.
    ///
    /// Known as:
    ///
    /// 1. hex: `#ff000000`
    /// 2. rgb: `rgb(255, 0, 0)`
    ///
    /// Because the implementation detail is on the web side and validating the value means extra
    /// constraints, use any format considered as available.
    Custom(String),
}
