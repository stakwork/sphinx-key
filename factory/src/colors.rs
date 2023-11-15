pub(crate) struct Rgb {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

pub(crate) const BLUE: Rgb = Rgb {
    r: 00,
    g: 00,
    b: 255,
};

pub(crate) const GREEN: Rgb = Rgb {
    r: 00,
    g: 255,
    b: 00,
};

pub(crate) const ORANGE: Rgb = Rgb {
    r: 255,
    g: 55,
    b: 00,
};

pub(crate) const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};
