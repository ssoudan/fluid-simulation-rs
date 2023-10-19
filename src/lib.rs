//! This is the main entry point for the WASM module.
pub mod simu;
pub mod utils;
pub mod visualization;

use std::cell::RefCell;
use std::rc::Rc;
use std::vec;

use simu::DrawOptions;
use simu::ObstacleType;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_time::Instant;

use crate::simu::Fluid;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

/// Run the simulation with the given parameters.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn run_with_selector(
    num_iters: u32,
    over_relaxation: f32,
    mut fluid: Fluid,
    canvas: web_sys::HtmlCanvasElement,
    scenario_selector: web_sys::HtmlSelectElement,
    pressure_checkbox: web_sys::HtmlInputElement,
    streamlines_checkbox: web_sys::HtmlInputElement,
    streamlines_num_segs: web_sys::HtmlInputElement,
    streamlines_spacing: web_sys::HtmlInputElement,
    in_vel: web_sys::HtmlInputElement,
    colormap_selector: web_sys::HtmlSelectElement,
    sim_to_canvas_ratio: u32,
) -> Result<(), JsValue> {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let previous_frame = Rc::new(RefCell::new(Instant::now()));

    let scenario = Rc::new(RefCell::new(None::<(String, f32)>));

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    *g.borrow_mut() = Some(Closure::new(move || {
        let now = Instant::now();
        let dt = now.duration_since(*previous_frame.borrow()).as_secs_f32();
        *previous_frame.borrow_mut() = now;

        if dt != 0.0 {
            // Update the fluid.
            fluid.simulate(dt, num_iters, over_relaxation);
        }

        let pressure = pressure_checkbox.checked();

        let streamlines = streamlines_checkbox.checked();

        let colormap_value = colormap_selector.value();

        let streamlines_num_segs = streamlines_num_segs.value_as_number() as usize;
        let streamlines_spacing = streamlines_spacing.value_as_number() as usize;

        let in_vel = in_vel.value_as_number() as f32;

        let options = DrawOptions {
            pressure,
            obstacle: true,
            streamlines,
            streamlines_num_segs,
            streamlines_spacing,
            colormap: colormap_value,
        };

        // What scenario are we in?
        let scenario_value = scenario_selector.value();

        // If the scenario is not set or has changed, update the fluid.
        let mut scenario = scenario.borrow_mut();
        match scenario.as_ref() {
            Some((sv, in_vel_)) if (sv == scenario_value.as_str()) && (*in_vel_ == in_vel) => {}
            _ => {
                scenario.replace((scenario_value.clone(), in_vel));

                match scenario_value.as_str() {
                    "rectangular" => {
                        fluid.clear_obstacles();
                        fluid.vortex_shedding(
                            in_vel,
                            vec![ObstacleType::Rectangular {
                                x: 0.2,
                                y: 0.5,
                                w: 0.1,
                                h: 0.3,
                            }],
                        );
                    }
                    _ => {
                        fluid.clear_obstacles();
                        fluid.vortex_shedding(
                            in_vel,
                            vec![ObstacleType::Circular {
                                x: 0.5,
                                y: 0.5,
                                r: 0.2,
                            }],
                        );
                    }
                }
            }
        }

        fluid
            .render(options, dt, sim_to_canvas_ratio, &context)
            .expect("draw failed");

        // Schedule ourself for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}
