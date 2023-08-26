pub mod simu;

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

// use std::f64;
// use wasm_bindgen::JsCast;

// #[wasm_bindgen(start)]
// pub fn start() {
//     let document = web_sys::window().unwrap().document().unwrap();
//     let canvas = document.get_element_by_id("canvas").unwrap();
//     let canvas: web_sys::HtmlCanvasElement = canvas
//         .dyn_into::<web_sys::HtmlCanvasElement>()
//         .map_err(|_| ())
//         .unwrap();

//     let context = canvas
//         .get_context("2d")
//         .unwrap()
//         .unwrap()
//         .dyn_into::<web_sys::CanvasRenderingContext2d>()
//         .unwrap();

//     context.begin_path();

//     // Draw the outer circle.
//     context
//         .arc(75.0, 75.0, 50.0, 0.0, f64::consts::PI * 2.0)
//         .unwrap();

//     // Draw the mouth.
//     context.move_to(110.0, 75.0);
//     context.arc(75.0, 75.0, 35.0, 0.0, f64::consts::PI).unwrap();

//     // Draw the left eye.
//     context.move_to(65.0, 65.0);
//     context
//         .arc(60.0, 65.0, 5.0, 0.0, f64::consts::PI * 2.0)
//         .unwrap();

//     // Draw the right eye.
//     context.move_to(95.0, 65.0);
//     context
//         .arc(90.0, 65.0, 5.0, 0.0, f64::consts::PI * 2.0)
//         .unwrap();

//     context.stroke();
// }

// #[wasm_bindgen]
// pub fn draw(
//     ctx: &CanvasRenderingContext2d,
//     width: u32,
//     height: u32,
//     real: f64,
//     imaginary: f64,
// ) -> Result<(), JsValue> {
//     // The real workhorse of this algorithm, generating pixel data
//     let c = Complex { real, imaginary };
//     let mut data = get_julia_set(width, height, c);
//     let data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&mut data), width, height)?;
//     ctx.put_image_data(&data, 0.0, 0.0)
// }

#[wasm_bindgen]
pub fn run(
    dt: f32,
    num_iters: u32,
    over_relaxation: f32,
    mut fluid: Fluid,
    canvas: web_sys::HtmlCanvasElement,
    sim_to_canvas_ratio: u32,
) -> Result<(), JsValue> {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    // let document = web_sys::window().unwrap().document().unwrap();
    // let canvas = document.get_element_by_id("canvas").unwrap();
    // let canvas: web_sys::HtmlCanvasElement = canvas
    //     .dyn_into::<web_sys::HtmlCanvasElement>()
    //     .map_err(|_| ())
    //     .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    *g.borrow_mut() = Some(Closure::new(move || {
        fluid.simulate(dt, num_iters, over_relaxation);

        let options = DrawOptions {
            pressure: true,
            obstacle: true,
            streamlines: true,
        };

        fluid
            .draw(options, dt, sim_to_canvas_ratio, &context)
            .expect("draw failed");

        // Schedule ourself for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}
