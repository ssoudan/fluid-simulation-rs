//! This is the main entry point for the WASM module.
pub mod simu;
pub mod utils;

use std::cell::RefCell;
use std::rc::Rc;

use simu::DrawOptions;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
    dt: f32,
    num_iters: u32,
    over_relaxation: f32,
    mut fluid: Fluid,
    canvas: web_sys::HtmlCanvasElement,
    scenario_selector: web_sys::HtmlSelectElement,
    pressure_checkbox: web_sys::HtmlInputElement,
    streamlines_checkbox: web_sys::HtmlInputElement,
    sim_to_canvas_ratio: u32,
) -> Result<(), JsValue> {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let scenario = Rc::new(RefCell::new(None::<String>));

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    *g.borrow_mut() = Some(Closure::new(move || {
        fluid.simulate(dt, num_iters, over_relaxation);

        let pressure = pressure_checkbox.checked();

        let streamlines = streamlines_checkbox.checked();

        let options = DrawOptions {
            pressure,
            obstacle: true,
            streamlines,
        };

        // What scenario are we in?
        let scenario_value = scenario_selector.value();

        // If the scenario is not set or has changed, update the fluid.
        let mut scenario = scenario.borrow_mut();
        match scenario.as_ref() {
            Some(sv) => {
                if sv != scenario_value.as_str() {
                    scenario.replace(scenario_value.clone());

                    match scenario_value.as_str() {
                        "tank" => {
                            fluid.clear_obstacles();
                            fluid.tank();
                        }
                        _ => {
                            fluid.clear_obstacles();
                            fluid.vortex_shedding();
                        }
                    }
                }
            }
            None => {
                scenario.replace(scenario_value.clone());

                match scenario_value.as_str() {
                    "tank" => {
                        fluid.clear_obstacles();
                        fluid.tank();
                    }
                    _ => {
                        fluid.clear_obstacles();
                        fluid.vortex_shedding();
                    }
                }
            }
        }

        fluid
            .draw(options, dt, sim_to_canvas_ratio, &context)
            .expect("draw failed");

        // Schedule ourself for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}
