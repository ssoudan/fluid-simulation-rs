import "./styles.css";

import('./pkg')
    .then(wasm => {
        // Get the scenario selector
        const scenario_selector = document.getElementById("scenario");

        // Get the pressure checkbox
        const pressure_checkbox = document.getElementById("pressure");

        // Get the streamlines checkbox
        const streamlines_checkbox = document.getElementById("streamlines");

        // Get the colormap selector
        const colormap_selector = document.getElementById("colormap");

        // Get the canvas element
        const simu_canvas = document.getElementById("canvas");

        // Get the canvas context
        const simu_context = simu_canvas.getContext("2d");

        // Get the width and height of the canvas
        const simu_width = simu_canvas.width;
        const simu_height = simu_canvas.height;

        // Clear the canvas
        simu_context.clearRect(0, 0, simu_width, simu_height);

        // Configure the simulation
        console.log("simu_canvas.width: " + simu_canvas.width + " simu_canvas.height: " + simu_canvas.height);

        var aspectRatio = simu_canvas.width / simu_canvas.height;
        console.log("aspectRatio: " + aspectRatio);

        const domainHeight = 1.0;
        var domainWidth = domainHeight * aspectRatio;
        console.log("domainWidth: " + domainWidth + " domainHeight: " + domainHeight);

        var h = domainHeight / 150.;

        var numX = Math.floor(domainWidth / h);
        var numY = Math.floor(domainHeight / h);
        console.log("numX: " + numX + " numY: " + numY);

        var sim_to_canvas_ratio = simu_canvas.width / numX;
        console.log("sim_to_canvas_ratio: " + sim_to_canvas_ratio);

        var density = 1000.0;

        const dt = 1.0 / 60.0;
        const numIters = 40;

        const overrelaxation = 1.9;

        const gravity = -9.81;

        // Create the fluid simulation
        const fluid = wasm.Fluid.create(gravity, numX, numY, h, density)

        // Setup the obstacles
        fluid.clear_obstacles();

        // Run the simulation
        wasm.run_with_selector(dt, numIters, overrelaxation, fluid,
            simu_canvas, scenario_selector,
            pressure_checkbox, streamlines_checkbox,
            colormap_selector,
            sim_to_canvas_ratio)

    })
    .catch(console.error);