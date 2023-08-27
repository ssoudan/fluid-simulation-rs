//! Visualization code

use std::convert::TryFrom;

use wasm_bindgen::{Clamped, JsValue};

pub(crate) struct Image {
    data: Vec<u8>,
    width: usize,
    height: usize,
    resolution: usize,
}

impl Image {
    pub(crate) fn new(width: usize, height: usize, resolution: usize) -> Self {
        let width = width * resolution;
        let height = height * resolution;

        let data = vec![0_u8; 4 * width * height];
        Self {
            data,
            width,
            height,
            resolution,
        }
    }

    /// color a resolution by resolution square at (i*resolution, j*resolution)
    pub(crate) fn paint(&mut self, i: usize, j: usize, color: [u8; 4]) {
        for ii in 0..self.resolution {
            for jj in 0..self.resolution {
                let i = i * self.resolution + ii;
                let j = j * self.resolution + jj;

                let index = 4 * (i + j * self.width);

                self.data[index] = color[0];
                self.data[index + 1] = color[1];
                self.data[index + 2] = color[2];
                self.data[index + 3] = color[3];
            }
        }
    }

    /// Returns the size of the image in pixels.
    pub(crate) fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}

impl TryFrom<Image> for web_sys::ImageData {
    type Error = JsValue;

    fn try_from(image: Image) -> Result<Self, Self::Error> {
        let Image {
            data,
            width,
            height,
            ..
        } = image;

        web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&data),
            width as u32,
            height as u32,
        )
    }
}

pub(crate) trait Colormap {
    fn get_color(&self, x: f32, min_: f32, max_: f32) -> [u8; 4];
}

pub(crate) struct JetColormap {}

impl Colormap for JetColormap {
    fn get_color(&self, x: f32, min_: f32, max_: f32) -> [u8; 4] {
        let x = f32::min(f32::max(x, min_), max_ - 0.0001);
        let d = max_ - min_;
        let x = if d == 0. { 0.5 } else { (x - min_) / d };
        let m = 0.25;
        let num = f32::floor(x / m);
        let s = (x - num * m) / m;

        let (r, g, b) = match num as u8 {
            0 => (0.0, s, 1.0),
            1 => (0.0, 1.0, 1. - s),
            2 => (s, 1.0, 0.),
            3 => (1.0, 1.0 - s, 0.0),
            4 => (1.0, 1.0 - s, 0.0),
            _ => panic!("should not happen"),
        };

        let r = (r * 255.0) as u8;
        let g = (g * 255.0) as u8;
        let b = (b * 255.0) as u8;
        [r, g, b, 255]
    }
}

pub(crate) struct CoolWarmColormap {}

impl Colormap for CoolWarmColormap {
    fn get_color(&self, x: f32, min_: f32, max_: f32) -> [u8; 4] {
        let x = f32::min(f32::max(x, min_), max_ - 0.0001);
        let d = max_ - min_;
        let x = if d == 0. { 0.5 } else { (x - min_) / d };

        let r = 0.5 * (1.0 + x);
        let b = 1.0 - r;
        let g = 1.0 - (r + b);

        let r = (r * 255.0) as u8;
        let g = (g * 255.0) as u8;
        let b = (b * 255.0) as u8;
        [r, g, b, 255]
    }
}

/// Rainbow colormap
pub(crate) struct RainbowColormap {}

impl Colormap for RainbowColormap {
    fn get_color(&self, x: f32, min_: f32, max_: f32) -> [u8; 4] {
        let x = f32::min(f32::max(x, min_), max_ - 0.0001);
        let d = max_ - min_;
        let x = if d == 0. { 0.5 } else { (x - min_) / d };

        let r = if x < 0.5 {
            1.0 - 2.0 * x
        } else {
            2.0 * (x - 0.5)
        };
        let g = if x < 0.5 { 2.0 * x } else { 2.0 * (1.0 - x) };
        let b = if x < 0.5 { 2.0 * x } else { 0.0 };

        let r = (r * 255.0) as u8;
        let g = (g * 255.0) as u8;
        let b = (b * 255.0) as u8;
        [r, g, b, 255]
    }
}

pub(crate) fn colormap(colormap: &str) -> Box<dyn Colormap> {
    match colormap {
        "jet" => Box::new(JetColormap {}),
        "coolwarm" => Box::new(CoolWarmColormap {}),
        "rainbow" => Box::new(RainbowColormap {}),
        "grayscale" => Box::new(GrayscaleColormap {}),
        _ => panic!("unknown colormap"),
    }
}

/// Grayscale colormap
pub(crate) struct GrayscaleColormap {}

impl Colormap for GrayscaleColormap {
    fn get_color(&self, x: f32, min_: f32, max_: f32) -> [u8; 4] {
        let x = f32::min(f32::max(x, min_), max_ - 0.0001);
        let d = max_ - min_;
        let x = if d == 0. { 0.5 } else { (x - min_) / d };

        let r = x;
        let g = x;
        let b = x;

        let r = (r * 255.0) as u8;
        let g = (g * 255.0) as u8;
        let b = (b * 255.0) as u8;
        [r, g, b, 255]
    }
}
