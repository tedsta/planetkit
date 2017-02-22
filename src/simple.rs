use std::sync::mpsc;

use piston_window::PistonWindow;

use slog;
use slog_term;

use window;
use app;

/// Create a new simple PlanetKit app and window.
///
/// Uses all default settings, and logs to standard output.
pub fn new() -> (app::App, PistonWindow) {
    // Set up logger to print to standard output.
    use slog::DrainExt;
    let drain = slog_term::streamer().compact().build().fuse();
    let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
    let log = root_log;

    // Event channel for camera system
    let (camera_input_sender, camera_input_receiver) = mpsc::channel();

    let mut window = window::make_window(&log);
    let mut app = app::App::new(&log, &mut window, camera_input_sender);

    // Set up input adapters.
    use cell_dweller;
    let (movement_input_sender, movement_input_receiver) = mpsc::channel();
    let movement_input_adapter = cell_dweller::MovementInputAdapter::new(movement_input_sender);
    app.add_input_adapter(Box::new(movement_input_adapter));

    let (mining_input_sender, mining_input_receiver) = mpsc::channel();
    let mining_input_adapter = cell_dweller::MiningInputAdapter::new(mining_input_sender);
    app.add_input_adapter(Box::new(mining_input_adapter));

    {
        let planner = app.planner();

        {
            // Register all component types.
            let world = planner.mut_world();
            world.register::<::cell_dweller::CellDweller>();
            world.register::<::render::Visual>();
            world.register::<::Spatial>();
            world.register::<::globe::Globe>();
            world.register::<::globe::ChunkView>();
            world.register::<::render::player_camera::ClientPlayer>();
        }

        // TODO: move _all_ other system initialization from `app.rs`
        // into here, and then back out into helper functions.

        use super::system_priority as prio;

        let movement_sys = cell_dweller::MovementSystem::new(
            movement_input_receiver,
            &log,
        );
        planner.add_system(movement_sys, "cd_movement", prio::CD_MOVEMENT);

        let mining_sys = cell_dweller::MiningSystem::new(
            mining_input_receiver,
            &log,
        );
        planner.add_system(mining_sys, "cd_mining", prio::CD_MINING);

        let physics_sys = cell_dweller::PhysicsSystem::new(
            &log,
            0.1, // Seconds between falls
        );
        planner.add_system(physics_sys, "cd_physics", prio::CD_PHYSICS);

        use globe;
        let chunk_sys = globe::ChunkSystem::new(
            &log,
        );
        planner.add_system(chunk_sys, "chunk", prio::CHUNK);
        let chunk_view_sys = globe::ChunkViewSystem::new(
            &log,
            0.05, // Seconds between geometry creation
        );
        planner.add_system(chunk_view_sys, "chunk_view", prio::CHUNK_VIEW);

        let camera_update_sys = ::render::player_camera::System::new(camera_input_receiver);
        planner.add_system(camera_update_sys, "camera_update", 50);
    }

    app.temp_remove_me_init();

    (app, window)
}
