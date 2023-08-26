import('./pkg')
    .then(wasm => {
        // const canvas = document.getElementById('drawing');
        // const ctx = canvas.getContext('2d');

        // const realInput = document.getElementById('real');
        // const imaginaryInput = document.getElementById('imaginary');
        // const renderBtn = document.getElementById('render');

        // renderBtn.addEventListener('click', () => {
        //     const real = parseFloat(realInput.value) || 0;
        //     const imaginary = parseFloat(imaginaryInput.value) || 0;
        //     wasm.draw(ctx, 600, 600, real, imaginary);
        // });

        // wasm.draw(ctx, 600, 600, -0.15, 0.65);

        // setup the simulation loop

        const simu_canvas = document.getElementById('canvas');

        const res = 75.;

        simu_canvas.focus();

        console.log("simu_canvas.width: " + simu_canvas.width + " simu_canvas.height: " + simu_canvas.height);

        var aspectRatio = simu_canvas.width / simu_canvas.height;
        console.log("aspectRatio: " + aspectRatio);

        const domainHeight = 1.0;
        var domainWidth = domainHeight * aspectRatio;
        console.log("domainWidth: " + domainWidth + " domainHeight: " + domainHeight);

        var h = domainHeight / res;

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

        const fluid = wasm.Fluid.create(gravity, numX, numY, h, density)

        fluid.clear_obstacles();
        // fluid.tank();
        fluid.vortex_shedding();

        wasm.run(dt, numIters, overrelaxation, fluid, simu_canvas, sim_to_canvas_ratio)

    })
    .catch(console.error);