//! Eulerian fluid simulation
//!
//! From https://www.youtube.com/watch?v=iKAVRgIrUOU&list=PL-GwXAGjZ9fUf_7_MiBbPuLSJVp_3Edmq&index=1&t=6s
//! Code from https://www.youtube.com/redirect?event=video_description&redir_token=QUFFLUhqazhqYnZnQVliZFVwSjdzMVdnSnpfbGJYdkRCZ3xBQ3Jtc0tueVZhRGl4TVdhM25Xa0JEcXRPcmNqNzVpR1VkX3FINzUzZktVY1IxS3I2MWpXNDJfdm9XeExDUTFlbUwwVDY5WW1rZkY4TkR1eE9mTWZIclpDU0ZaVFBIM19qNGdxTjBfZGZGTU9STFVwU1V2a2JmOA&q=https%3A%2F%2Fmatthias-research.github.io%2Fpages%2FtenMinutePhysics%2Findex.html
use std::{convert::TryInto, vec};

use crate::visualization;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

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

    fn extrapolate(&mut self) {
        let n = self.num_y;

        for i in 0..self.num_x {
            self.u[i * n /* + 0*/] = self.u[i * n + 1];
            self.u[i * n + self.num_y - 1] = self.u[i * n + self.num_y - 2];
        }
        for j in 0..self.num_y {
            self.v[/*0 * n + */ j] = self.v[/* 1* */ n + j];
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

        sx * sy * f[x0 as usize * n + y0 as usize]
            + tx * sy * f[x1 as usize * n + y0 as usize]
            + tx * ty * f[x1 as usize * n + y1 as usize]
            + sx * ty * f[x0 as usize * n + y1 as usize]
    }

    fn avg_u(&self, i: usize, j: usize) -> f32 {
        let n = self.num_y;
        (self.u[i * n + j - 1]
            + self.u[i * n + j]
            + self.u[(i + 1) * n + j - 1]
            + self.u[(i + 1) * n + j])
            * 0.25
    }

    fn avg_v(&self, i: usize, j: usize) -> f32 {
        let n = self.num_y;
        (self.v[(i - 1) * n + j]
            + self.v[i * n + j]
            + self.v[(i - 1) * n + j + 1]
            + self.v[i * n + j + 1])
            * 0.25
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

    /// Return the pressure field
    pub fn pressure(&self) -> Vec<f32> {
        self.p.clone()
    }

    /// Render the simulation on the given canvas.
    pub fn render(
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

        let mut image =
            visualization::Image::new(self.num_x - 2, self.num_y - 2, sim_to_canvas_ratio as usize);

        // pressure
        if options.pressure {
            let colormap: Box<dyn visualization::Colormap> =
                visualization::colormap(options.colormap.as_str());

            for i in 1..self.num_x - 1 {
                for j in 1..self.num_y - 1 {
                    let p = self.p[i * n + j];
                    let color = colormap.get_color(p, min_p, max_p);

                    image.paint(i - 1, j - 1, color);
                }
            }
        }

        // draw the obstacle
        if options.obstacle {
            for i in 1..self.num_x - 1 {
                for j in 1..self.num_y - 1 {
                    if self.s[i * n + j] == 0. {
                        let color = [0, 0, 0, 255];

                        image.paint(i - 1, j - 1, color);
                    }
                }
            }
        }

        let (_width, height) = image.size();

        let data = image.try_into()?;
        let r = ctx.put_image_data(&data, 0.0, 0.0);

        let text = format!(
            "min: {:>8.1}\tmax: {:>8.1}\t{:>8.1} fps",
            min_p,
            max_p,
            1. / dt
        );

        let _ = ctx.fill_text(&text, 12., 12.);

        // draw stream line
        if options.streamlines {
            let h = self.h;
            let h2 = 0.5 * h;

            let real_to_canvas = sim_to_canvas_ratio as f64 / self.h as f64;

            let seg_len = 0.01;

            ctx.set_stroke_style(&"rgba(255, 0, 0, 1.)".into());
            ctx.set_line_width(0.8);

            for i in (1..self.num_x - 1).step_by(options.streamlines_spacing) {
                for j in (1..self.num_y - 1).step_by(options.streamlines_spacing) {
                    // center of the cell - real world coordinates
                    let mut x = i as f32 * h + h2;
                    let mut y = j as f32 * h + h2;

                    // center of the cell - canvas coordinates
                    let cx = x as f64 * real_to_canvas;
                    let cy = height as f64 - y as f64 * real_to_canvas;
                    ctx.begin_path();
                    ctx.move_to(cx, cy);

                    for _k in 0..options.streamlines_num_segs {
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

    /// Simulate the fluid for the given time step.
    pub fn simulate(&mut self, dt: f32, num_iters: u32, over_relaxation: f32) {
        self.integrate(dt, self.gravity);

        self.p.fill(0.);
        self.solve_incompressibility(over_relaxation, num_iters, dt);

        self.extrapolate();
        self.advect_velocity(dt);
        self.advect_smoke(dt);
    }

    pub fn add_obstacle(&mut self, obstacle: impl Obstacle) {
        const FLUID: f32 = 1.0;
        const OBSTACLE: f32 = 0.0;

        let n = self.num_y;

        let vx = 0.0;
        let vy = 0.0;

        for i in 1..self.num_x - 1 {
            for j in 1..self.num_y - 1 {
                let mut s = FLUID;

                if obstacle.is_inside((i as f32 + 0.5) * self.h, (j as f32 + 0.5) * self.h) {
                    s = OBSTACLE;

                    self.m[i * n + j] = 1.0;

                    self.u[i * n + j] = vx;
                    self.u[(i + 1) * n + j] = vx;
                    self.v[i * n + j] = vy;
                    self.v[i * n + j + 1] = vy;
                }

                self.s[i * n + j] = s;
            }
        }
    }

    pub fn add_obstacles(&mut self, obstacles: Vec<Box<dyn Obstacle>>) {
        const FLUID: f32 = 1.0;
        const OBSTACLE: f32 = 0.0;

        let n = self.num_y;

        let vx = 0.0;
        let vy = 0.0;

        for i in 1..self.num_x - 1 {
            for j in 1..self.num_y - 1 {
                let mut s = FLUID;

                for obstacle in &obstacles {
                    if obstacle.is_inside((i as f32 + 0.5) * self.h, (j as f32 + 0.5) * self.h) {
                        s = OBSTACLE;

                        self.m[i * n + j] = 1.0;

                        self.u[i * n + j] = vx;
                        self.u[(i + 1) * n + j] = vx;
                        self.v[i * n + j] = vy;
                        self.v[i * n + j + 1] = vy;
                    }
                }

                self.s[i * n + j] = s;
            }
        }
    }

    /// flow in a pipe and around obstacles with no gravity
    pub fn vortex_shedding(&mut self, obstacles: Vec<ObstacleType>) {
        const FLUID: f32 = 1.0;
        const OBSTACLE: f32 = 0.0;

        let in_vel = 2.0;

        let n = self.num_y;

        for i in 0..self.num_x {
            for j in 0..self.num_y {
                let mut s = FLUID;

                // borders
                if i == 0 || j == self.num_y - 1 || j == 0 {
                    s = OBSTACLE;
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
            self.m[n + j] = 0.;
        }

        self.gravity = 0.;

        self.add_obstacles(obstacles.into_iter().map(|x| x.into()).collect());
    }

    // TODO(ssoudan) NACA profile

    /// add a rectangular obstacle
    pub fn add_rectangular_obstacle(&mut self, x: f32, y: f32, w: f32, h: f32) {
        const FLUID: f32 = 1.0;
        const OBSTACLE: f32 = 0.0;

        let n = self.num_y;

        let vx = 0.0;
        let vy = 0.0;

        for i in 1..self.num_x - 1 {
            for j in 1..self.num_y - 1 {
                // obstacle
                self.s[i * n + j] = FLUID;

                let dx = (i as f32 + 0.5) * self.h - x;
                let dy = (j as f32 + 0.5) * self.h - y;

                if dx.abs() < w && dy.abs() < h {
                    self.s[i * n + j] = OBSTACLE;

                    self.m[i * n + j] = 1.0;

                    self.u[i * n + j] = vx;
                    self.u[(i + 1) * n + j] = vx;
                    self.v[i * n + j] = vy;
                    self.v[i * n + j + 1] = vy;
                }
            }
        }
    }

    /// add circular obstacle
    pub fn add_circular_obstacle(&mut self, x: f32, y: f32, r: f32) {
        const FLUID: f32 = 1.0;
        const OBSTACLE: f32 = 0.0;

        let vx = 0.0;
        let vy = 0.0;

        let n = self.num_y;

        for i in 1..self.num_x - 1 {
            for j in 1..self.num_y - 1 {
                // obstacle
                self.s[i * n + j] = FLUID;

                let dx = (i as f32 + 0.5) * self.h - x;
                let dy = (j as f32 + 0.5) * self.h - y;

                let d = (dx * dx + dy * dy).sqrt();

                if d < r {
                    self.s[i * n + j] = OBSTACLE;

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

#[wasm_bindgen]
impl Fluid {
    pub fn create(gravity: f32, num_x: usize, num_y: usize, h: f32, density: f32) -> Fluid {
        let num_x = num_x + 2; // 2 border cells
        let num_y = num_y + 2;

        let num_cells = num_x * num_y;
        let u = vec![0.0; num_cells];
        let v = vec![0.0; num_cells];
        let new_u = vec![0.0; num_cells];
        let new_v = vec![0.0; num_cells];
        let p = vec![0.0; num_cells];
        let s = vec![0.0; num_cells];
        let m = vec![1.0; num_cells];
        let new_m = vec![0.0; num_cells];
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
        const FLUID: f32 = 1.0;

        self.s.fill(FLUID);
    }
}

pub struct DrawOptions {
    pub pressure: bool,
    pub obstacle: bool,
    pub streamlines: bool,
    pub streamlines_spacing: usize,
    pub streamlines_num_segs: usize,
    pub colormap: String,
}

/// Obstacle type
pub enum ObstacleType {
    /// Rectangular obstacle
    Rectangular { x: f32, y: f32, w: f32, h: f32 },
    /// Circular obstacle
    Circular { x: f32, y: f32, r: f32 },
}

impl From<ObstacleType> for Box<dyn Obstacle> {
    fn from(obstacle: ObstacleType) -> Self {
        match obstacle {
            ObstacleType::Rectangular { x, y, w, h } => {
                Box::new(RectangularObstacle::new(x, y, w, h))
            }
            ObstacleType::Circular { x, y, r } => Box::new(CircularObstacle::new(x, y, r)),
        }
    }
}

/// Obstacle
pub trait Obstacle {
    /// Return true if the given point is inside the obstacle
    fn is_inside(&self, x: f32, y: f32) -> bool;
}

/// Rectangular obstacle
pub(crate) struct RectangularObstacle {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl RectangularObstacle {
    /// Create a new rectangular obstacle
    #[inline]
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> RectangularObstacle {
        RectangularObstacle { x, y, w, h }
    }
}

impl Obstacle for RectangularObstacle {
    #[inline]
    fn is_inside(&self, x: f32, y: f32) -> bool {
        let dx = x - self.x;
        let dy = y - self.y;

        dx.abs() < self.w && dy.abs() < self.h
    }
}

/// Circular obstacle
pub(crate) struct CircularObstacle {
    x: f32,
    y: f32,
    r: f32,
}

impl CircularObstacle {
    /// Create a new circular obstacle
    #[inline]
    pub fn new(x: f32, y: f32, r: f32) -> CircularObstacle {
        CircularObstacle { x, y, r }
    }
}

impl Obstacle for CircularObstacle {
    #[inline]
    fn is_inside(&self, x: f32, y: f32) -> bool {
        let dx = x - self.x;
        let dy = y - self.y;

        let d = (dx * dx + dy * dy).sqrt();

        d < self.r
    }
}
