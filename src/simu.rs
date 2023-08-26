//! Eulerian fluid simulation
//!
//! From https://www.youtube.com/watch?v=iKAVRgIrUOU&list=PL-GwXAGjZ9fUf_7_MiBbPuLSJVp_3Edmq&index=1&t=6s
//! Code from https://www.youtube.com/redirect?event=video_description&redir_token=QUFFLUhqazhqYnZnQVliZFVwSjdzMVdnSnpfbGJYdkRCZ3xBQ3Jtc0tueVZhRGl4TVdhM25Xa0JEcXRPcmNqNzVpR1VkX3FINzUzZktVY1IxS3I2MWpXNDJfdm9XeExDUTFlbUwwVDY5WW1rZkY4TkR1eE9mTWZIclpDU0ZaVFBIM19qNGdxTjBfZGZGTU9STFVwU1V2a2JmOA&q=https%3A%2F%2Fmatthias-research.github.io%2Fpages%2FtenMinutePhysics%2Findex.html
use std::vec;

use wasm_bindgen::{prelude::*, Clamped};
use web_sys::console;
use web_sys::{CanvasRenderingContext2d, ImageData};

pub struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(name: &'a str) -> Timer<'a> {
        console::time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        console::time_end_with_label(self.name);
    }
}

/// A fluid simulation.
///
/// [u] and [v] use a staggered grid, where [u] is at the center of
/// the cell in the x-direction and [v] is at the center of the cell
/// in the y-direction.
///
/// [new_u] and [new_v] are the velocities at t+dt.
///
///      v_{i,j+1}
/// -----X-----
/// |         |
/// |         |
/// X u_{i,j} X u_{i+1,j}
/// |         |
/// |  v_{i,j}|
/// -----X-----
///
///      u[num_y-1]
///      ...       ...
/// Y  ^ u[2]      u[num_y+2]    -j-   u[i*num_y+j]
///    | u[1]      u[num_y+1]               |
///    | u[0]      u[num_y]                 i
///    0-----> X                            |
#[wasm_bindgen]
pub struct Fluid {
    /// gravity
    gravity: f32,

    /// density of the fluid
    density: f32,

    num_x: usize,
    num_y: usize,

    // cell size (in meters)
    h: f32,

    /// x-component of velocity at t
    /// u_{i,j} = u[i * num_y + j ]
    u: Vec<f32>,
    /// y-component of velocity at t
    /// v_{i,j} = v[i * num_y + j ]
    v: Vec<f32>,

    /// x-component of velocity at t+dt
    new_u: Vec<f32>,
    /// y-component of velocity at t+dt
    new_v: Vec<f32>,

    /// pressure field
    p: Vec<f32>,

    /// obstacle field
    /// s == 0 => obstacle
    /// s == 1 => fluid
    s: Vec<f32>,

    /// smoke field at t
    m: Vec<f32>,
    /// smoke field at t+dt
    new_m: Vec<f32>,
}

enum Field {
    U,
    V,
    S,
}

impl Fluid {
    fn integrate(&mut self, dt: f32, gravity: f32) {
        let n = self.num_y;

        // ignore i == 0 but not i == num_x-1
        for i in 1..self.num_x {
            // ignore j == 0 and j == num_y-1 as well
            for j in 1..(self.num_y - 1) {
                // if it is not an obstacle and the cell below is not an obstacle
                if self.s[i * n + j] != 0. && self.s[i * n + j - 1] != 0. {
                    self.v[i * n + j] += dt * gravity;
                }
            }
        }
    }

    fn solve_incompressibility(&mut self, over_relaxation: f32, num_iters: u32, dt: f32) {
        let n = self.num_y;

        let cp = self.density * self.h / dt;

        for _ in 0..num_iters {
            // iterate over the interior cells
            for i in 1..self.num_x - 1 {
                for j in 1..self.num_y - 1 {
                    // skip solid cells
                    if self.s[i * n + j] == 0. {
                        continue;
                    }

                    // are neighbors solid?
                    let sx0 = self.s[(i - 1) * n + j];
                    let sx1 = self.s[(i + 1) * n + j];
                    let sy0 = self.s[i * n + j - 1];
                    let sy1 = self.s[i * n + j + 1];

                    // number of neighbors we can exchange flow with
                    let s = sx0 + sx1 + sy0 + sy1;

                    if s == 0. {
                        continue;
                    }

                    // divergence of velocity in staggered grid
                    let div = self.u[(i + 1) * n + j] - self.u[i * n + j] + self.v[i * n + j + 1]
                        - self.v[i * n + j];

                    // distribute the divergence to the neighbors
                    let p = -div / s;
                    // accelerate convergence with over relaxation
                    let p = p * over_relaxation;

                    // update the pressure
                    self.p[i * n + j] += cp * p;

                    // correct velocity field
                    self.u[i * n + j] -= p * sx0;
                    self.u[(i + 1) * n + j] += p * sx1;
                    self.v[i * n + j] -= p * sy0;
                    self.v[i * n + j + 1] += p * sy1;
                }
            }
        }
    }

    // TODO(ssoudan) document
    fn extrapolate(&mut self) {
        let n = self.num_y;

        for i in 0..self.num_x {
            self.u[i * n + 0] = self.u[i * n + 1];
            self.u[i * n + self.num_y - 1] = self.u[i * n + self.num_y - 2];
        }
        for j in 0..self.num_y {
            self.v[0 * n + j] = self.v[1 * n + j];
            self.v[(self.num_x - 1) * n + j] = self.v[(self.num_x - 2) * n + j];
        }
    }

    fn sample_field(&self, x: f32, y: f32, field: Field) -> f32 {
        let n = self.num_y;
        let h = self.h;
        let h1 = 1.0 / h;
        let h2 = 0.5 * h;

        let x = f32::max(f32::min(x, self.num_x as f32 * h), h);
        let y = f32::max(f32::min(y, self.num_y as f32 * h), h);

        let mut dx = 0.;
        let mut dy = 0.;

        let f = match field {
            Field::U => {
                dy = h2;
                &self.u
            }
            Field::V => {
                dx = h2;
                &self.v
            }
            Field::S => {
                dx = h2;
                dy = h2;
                &self.m
            }
        };

        let x0 = f32::min(f32::floor((x - dx) * h1), self.num_x as f32 - 1.);
        let tx = ((x - dx) - x0 * h) * h1;
        let x1 = f32::min(x0 + 1., self.num_x as f32 - 1.);

        let y0 = f32::min(f32::floor((y - dy) * h1), self.num_y as f32 - 1.);
        let ty = ((y - dy) - y0 * h) * h1;
        let y1 = f32::min(y0 + 1., self.num_y as f32 - 1.);

        let sx = 1. - tx;
        let sy = 1. - ty;

        let val = sx * sy * f[x0 as usize * n + y0 as usize]
            + tx * sy * f[x1 as usize * n + y0 as usize]
            + tx * ty * f[x1 as usize * n + y1 as usize]
            + sx * ty * f[x0 as usize * n + y1 as usize];

        val
    }

    fn avg_u(&self, i: usize, j: usize) -> f32 {
        let n = self.num_y;
        let u = (self.u[i * n + j - 1]
            + self.u[i * n + j]
            + self.u[(i + 1) * n + j - 1]
            + self.u[(i + 1) * n + j])
            * 0.25;
        u
    }

    fn avg_v(&self, i: usize, j: usize) -> f32 {
        let n = self.num_y;
        let v = (self.v[(i - 1) * n + j]
            + self.v[i * n + j]
            + self.v[(i - 1) * n + j + 1]
            + self.v[i * n + j + 1])
            * 0.25;
        v
    }

    fn advect_velocity(&mut self, dt: f32) {
        self.new_u = self.u.clone();
        self.new_v = self.v.clone();

        let n = self.num_y;
        let h = self.h;
        let h2 = 0.5 * h;

        for i in 1..self.num_x {
            for j in 1..self.num_y {
                // u component
                if self.s[i * n + j] != 0. && self.s[(i - 1) * n + j] != 0. && j < self.num_y - 1 {
                    let x = i as f32 * h;
                    let y = j as f32 * h + h2;

                    let u = self.u[i * n + j];
                    let v = self.avg_v(i, j);

                    // backward step
                    let x = x - dt * u;
                    let y = y - dt * v;

                    // sample the velocity field
                    let u = self.sample_field(x, y, Field::U);

                    // update the velocity field
                    self.new_u[i * n + j] = u;
                }

                // v component
                if self.s[i * n + j] != 0. && self.s[i * n + j - 1] != 0. && i < self.num_x - 1 {
                    let x = i as f32 * h + h2;
                    let y = j as f32 * h;

                    let u = self.avg_u(i, j);
                    let v = self.v[i * n + j];

                    // backward step
                    let x = x - dt * u;
                    let y = y - dt * v;

                    // sample the velocity field
                    let v = self.sample_field(x, y, Field::V);

                    // update the velocity field
                    self.new_v[i * n + j] = v;
                }
            }
        }

        self.u = self.new_u.clone();
        self.v = self.new_v.clone();
    }

    fn advect_smoke(&mut self, dt: f32) {
        self.new_m = self.m.clone();

        let n = self.num_y;
        let h = self.h;
        let h2 = 1.0 / (2.0 * h);

        for i in 1..self.num_x - 1 {
            for j in 1..self.num_y - 1 {
                if self.s[i * n + j] != 0. {
                    let u = (self.u[i * n + j] + self.u[(i + 1) * n + j]) * 0.5;
                    let v = (self.v[i * n + j] + self.v[i * n + j + 1]) * 0.5;

                    let x = i as f32 * h + h2 - dt * u;
                    let y = j as f32 * h + h2 - dt * v;

                    // update the velocity field
                    self.new_m[i * n + j] = self.sample_field(x, y, Field::S);
                }
            }
        }

        self.m = self.new_m.clone();
    }

    pub fn pressure(&self) -> Vec<f32> {
        self.p.clone()
    }

    pub fn draw(
        &self,
        options: DrawOptions,
        dt: f32,
        sim_to_canvas_ratio: u32,
        ctx: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        // let _timer = Timer::new("Fluid::draw");

        let data = self.pressure();

        let (min_p, max_p) = data.iter().fold((f32::MAX, f32::MIN), |(min, max), &x| {
            (min.min(x), max.max(x))
        });

        let n = self.num_y;

        let mut image = Image::new(self.num_x - 2, self.num_y - 2, sim_to_canvas_ratio as usize);

        // pressure
        if options.pressure {
            for i in 1..self.num_x - 1 {
                for j in 1..self.num_y - 1 {
                    let p = self.p[i * n + j];
                    let color = get_sci_color(p, min_p, max_p);

                    image.paint(i - 1, j - 1, color);
                    // FIXME(ssoudan) scaling between simulation and canvas - HiDPI?
                }
            }
        }

        // draw the obstacle
        if options.obstacle {
            for i in 1..self.num_x - 1 {
                for j in 1..self.num_y - 1 {
                    if self.s[i * n + j] == 0. {
                        let color = [0, 0, 0, 255];

                        // FIXME(ssoudan) scaling between simulation and canvas - HiDPI?
                        image.paint(i - 1, j - 1, color);
                    }
                }
            }
        }

        let width = image.width as u32;
        let height = image.height as u32;

        let mut data = image.data;

        let data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&mut data), width, height)?;
        let r = ctx.put_image_data(&data, 0.0, 0.0);

        let text = format!("min: {:.2} max: {:.2} - {:.2} fps", min_p, max_p, 1. / dt);

        let _ = ctx.fill_text(&text, 12., 12.);

        // draw stream line
        if options.streamlines {
            let h = self.h;
            let h2 = 0.5 * h;

            let real_to_canvas = sim_to_canvas_ratio as f64 / self.h as f64;

            const NUM_SEGS: u32 = 10;
            let seg_len = 0.01;

            ctx.set_stroke_style(&"rgba(255, 0, 0, 1.)".into());
            ctx.set_line_width(1.0);

            for i in (1..self.num_x - 1).step_by(5) {
                for j in (1..self.num_y - 1).step_by(5) {
                    // center of the cell - real world coordinates
                    let mut x = i as f32 * h + h2;
                    let mut y = j as f32 * h + h2;

                    // center of the cell - canvas coordinates
                    let cx = x as f64 * real_to_canvas;
                    let cy = height as f64 - y as f64 * real_to_canvas;
                    ctx.begin_path();
                    ctx.move_to(cx, cy);

                    for _k in 0..NUM_SEGS {
                        let u = self.sample_field(x, y, Field::U);
                        let v = self.sample_field(x, y, Field::V);

                        let l = (u * u + v * v).sqrt();

                        // next point - real world coordinates
                        x += u / l * seg_len;
                        y += v / l * seg_len;

                        if x > self.num_x as f32 * h {
                            break;
                        }

                        let cx = x as f64 * real_to_canvas;
                        let cy = height as f64 - y as f64 * real_to_canvas;
                        ctx.line_to(cx, cy);
                    }
                    ctx.stroke();
                }
            }
        }

        r
    }

    pub fn simulate(&mut self, dt: f32, num_iters: u32, over_relaxation: f32) {
        self.integrate(dt, self.gravity);

        self.p.fill(0.);
        self.solve_incompressibility(over_relaxation, num_iters, dt);

        self.extrapolate();
        self.advect_velocity(dt);
        self.advect_smoke(dt);
    }
}

#[wasm_bindgen]
impl Fluid {
    pub fn create(gravity: f32, num_x: usize, num_y: usize, h: f32, density: f32) -> Fluid {
        let num_x = num_x + 2; // 2 border cells
        let num_y = num_y + 2;

        let num_cells = num_x * num_y;
        let u = vec![0.0; num_cells as usize];
        let v = vec![0.0; num_cells as usize];
        let new_u = vec![0.0; num_cells as usize];
        let new_v = vec![0.0; num_cells as usize];
        let p = vec![0.0; num_cells as usize];
        let s = vec![0.0; num_cells as usize];
        let m = vec![1.0; num_cells as usize];
        let new_m = vec![0.0; num_cells as usize];
        Fluid {
            gravity,
            density,
            num_x,
            num_y,
            h,
            u,
            v,
            new_u,
            new_v,
            p,
            s,
            m,
            new_m,
        }
    }

    /// clear obstacles
    pub fn clear_obstacles(&mut self) {
        self.s.fill(1.);
    }

    pub fn tank(&mut self) {
        let n = self.num_y;
        for i in 0..self.num_x {
            for j in 0..self.num_y {
                let mut s = 1.0; // fluid

                if i == 0 || i == self.num_x - 1 || j == 0 {
                    s = 0.0; // obstacle
                }

                self.s[i * n + j] = s;
            }
        }
    }

    pub fn vortex_shedding(&mut self) {
        let in_vel = 2.0;

        let n = self.num_y;

        for i in 0..self.num_x {
            for j in 0..self.num_y {
                let mut s = 1.0; // fluid

                if i == 0 || j == self.num_y - 1 || j == 0 {
                    s = 0.0; // obstacle
                }
                self.s[i * n + j] = s;

                if i == 1 {
                    self.u[i * n + j] = in_vel;
                }
            }
        }

        let pipe_h = 0.1 * self.num_y as f32;
        let min_j = f32::floor(0.5 * self.num_y as f32 - 0.5 * pipe_h) as usize;
        let max_j = f32::floor(0.5 * self.num_y as f32 + 0.5 * pipe_h) as usize;

        for j in min_j..max_j {
            self.m[1 * n + j] = 0.;
        }

        self.gravity = 0.;
        self.set_obstacle(0.4, 0.5, 0.3)
    }

    // TODO(ssoudan) NACA profile

    /// set obstacles
    pub fn set_obstacle(&mut self, x: f32, y: f32, r: f32) {
        let vx = 0.0;
        let vy = 0.0;

        let n = self.num_y;

        for i in 1..self.num_x - 1 {
            for j in 1..self.num_y - 1 {
                // obstacle
                self.s[i * n + j] = 1.0;

                let dx = (i as f32 + 0.5) * self.h - x;
                let dy = (j as f32 + 0.5) * self.h - y;

                let d = (dx * dx + dy * dy).sqrt();

                if d < r as f32 {
                    self.s[i * n + j] = 0.0;

                    self.m[i * n + j] = 1.0;

                    self.u[i * n + j] = vx;
                    self.u[(i + 1) * n + j] = vx;
                    self.v[i * n + j] = vy;
                    self.v[i * n + j + 1] = vy;
                }
            }
        }
    }
}

pub struct DrawOptions {
    pub pressure: bool,
    pub obstacle: bool,
    pub streamlines: bool,
}

struct Image {
    data: Vec<u8>,
    width: usize,
    height: usize,
    resolution: usize,
}

impl Image {
    fn new(width: usize, height: usize, resolution: usize) -> Self {
        let width = width * resolution;
        let height = height * resolution;

        let data = vec![0 as u8; 4 * width * height];
        Self {
            data,
            width,
            height,
            resolution,
        }
    }

    /// color a resolution by resolution square at (i*resolution, j*resolution)
    fn paint(&mut self, i: usize, j: usize, color: [u8; 4]) {
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
}

fn get_sci_color(x: f32, min_: f32, max_: f32) -> [u8; 4] {
    let x = f32::min(f32::max(x, min_), max_ - 0.0001);
    let d = max_ - min_;
    let x = if d == 0. { 0.5 } else { (x - min_) / d };
    let m = 0.25;
    let num = f32::floor(x / m);
    let s = (x - num * m) / m;

    let (r, g, b) = match num as u8 {
        0 => (1.0, s, 0.0),
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
