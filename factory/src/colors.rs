pub(crate) struct RGB {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

pub(crate) const BLUE: RGB = RGB {
    r: 00,
    g: 00,
    b: 255,
};

pub(crate) const GREEN: RGB = RGB {
    r: 00,
    g: 255,
    b: 00,
};

pub(crate) const ORANGE: RGB = RGB {
    r: 255,
    g: 55,
    b: 00,
};

pub(crate) const WHITE: RGB = RGB {
    r: 255,
    g: 255,
    b: 255,
};
