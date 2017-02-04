use std::sync::{ Arc, Mutex, mpsc };
use piston_window::PistonWindow;
use piston::input::{ self, UpdateArgs, RenderArgs };
use slog::Logger;
use gfx;
use gfx_device_gl;
use specs;

use render;
use render::{ Visual, Mesh, MeshRepository };
use types::*;
use globe;
use cell_dweller;

fn get_projection(w: &PistonWindow) -> [[f32; 4]; 4] {
    use piston::window::Window;
    use camera_controllers::CameraPerspective;

    let draw_size = w.window.draw_size();
    CameraPerspective {
        fov: 90.0, near_clip: 0.01, far_clip: 100.0,
        aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32)
    }.projection()
}

pub struct App {
    t: TimeDelta,
    log: Logger,
    planner: specs::Planner<TimeDelta>,
    encoder_channel: render::EncoderChannel<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    movement_input_sender: mpsc::Sender<cell_dweller::MovementEvent>,
    mining_input_sender: mpsc::Sender<cell_dweller::MiningEvent>,
    // TEMP: Share with rendering system until the rendering system
    // is smart enough to take full ownership of it.
    projection: Arc<Mutex<[[f32; 4]; 4]>>,
    camera_input_sender: mpsc::Sender<input::Event>,
    factory: gfx_device_gl::Factory,
    output_color: gfx::handle::RenderTargetView<gfx_device_gl::Resources, (gfx::format::R8_G8_B8_A8, gfx::format::Srgb)>,
    output_stencil: gfx::handle::DepthStencilView<gfx_device_gl::Resources, (gfx::format::D24_S8, gfx::format::Unorm)>,
    mesh_repo: Arc<Mutex<MeshRepository<gfx_device_gl::Resources>>>,
}

impl App {
    pub fn new(parent_log: &Logger, window: &PistonWindow) -> App {
        use camera_controllers::{
            FirstPersonSettings,
            FirstPerson,
        };
        use ::Spatial;

        // Rendering system, with bi-directional channel to pass
        // encoder back and forth between this thread (which owns
        // the graphics device) and any number of game threads managed by Specs.
        let (render_sys_send, device_recv) = mpsc::channel();
        let (device_send, render_sys_recv) = mpsc::channel();
        let render_sys_encoder_channel = render::EncoderChannel {
            sender: render_sys_send,
            receiver: render_sys_recv,
        };
        let device_encoder_channel = render::EncoderChannel {
            sender: device_send,
            receiver: device_recv,
        };

        // Shove two encoders into the channel circuit.
        // This gives us "double-buffering" by having two encoders in flight.
        // This way the render system will always be able to populate
        // an encoder, even while this thread is busy flushing one
        // to the video card.
        //
        // (Note: this is separate from the double-buffering of the
        // output buffers -- this is the command buffer that we can fill
        // up with drawing commands _before_ flushing the whole thing to
        // the video card in one go.)
        let enc1 = window.encoder.clone_empty();
        let enc2 = window.encoder.clone_empty();
        // TODO: this carefully sending one encoder to each
        // channel is only because I'm temporarily calling
        // the rendering system synchronously until I get
        // around to turning it into a Specs system. Juggling like
        // this prevents deadlock.
        render_sys_encoder_channel.sender.send(enc1).unwrap();
        device_encoder_channel.sender.send(enc2).unwrap();

        let log = parent_log.new(o!());

        let projection = Arc::new(Mutex::new(get_projection(window)));
        let camera = Camera::new([0.0, 0.0, 0.0]);

        let mut mesh_repo = MeshRepository::new(
            window.output_color.clone(),
            window.output_stencil.clone(),
            &log,
        );

        let (movement_input_sender, movement_input_receiver) = mpsc::channel();
        let movement_sys = cell_dweller::MovementSystem::new(
            movement_input_receiver,
            &log,
        );

        let (mining_input_sender, mining_input_receiver) = mpsc::channel();
        let mining_sys = cell_dweller::MiningSystem::new(
            mining_input_receiver,
            &log,
        );

        // Create SPECS world and, system execution planner
        // for it with two threads.
        //
        // This manages execution of all game systems,
        // i.e. the interaction between sets of components.
        let mut world = specs::World::new();
        world.register::<render::player_camera::ClientPlayer>();
        world.register::<cell_dweller::CellDweller>();
        world.register::<render::Visual>();
        world.register::<Spatial>();
        world.register::<globe::Globe>();
        world.register::<globe::ChunkView>();

        world.add_resource(camera);

        // Add some things to the world.

        // Make globe and create a mesh for each of its chunks.
        //
        // TODO: don't bake this into the generic app!
        let globe = globe::Globe::new_example(&log);

        // Find globe surface and put player character on it.
        use globe::{ CellPos, Dir };
        use globe::chunk::Material;
        let mut guy_pos = CellPos::default();
        guy_pos = globe.find_lowest_cell_containing(guy_pos, Material::Air)
            .expect("Uh oh, there's something wrong with our globe.");
        let factory = &mut window.factory.clone();
        let axes_mesh = render::make_axes_mesh(
            factory,
            &mut mesh_repo,
        );
        let snowman_mesh = render::make_obj_mesh(
            "assets/models/snowman.obj",
            "assets/models/snowman.mtl",
            0.01,
            factory,
            &mut mesh_repo,
        );
        let mut cell_dweller_visual = render::Visual::new_empty();
        cell_dweller_visual.set_mesh_handle(snowman_mesh);
        let globe_spec = globe.spec();
        // First add the globe to the world so we can get a
        // handle on its entity.
        let globe_entity = world.create_now()
            .with(globe)
            .build();
        world.create_now()
            .with(render::player_camera::ClientPlayer)
            .with(cell_dweller::CellDweller::new(
                guy_pos,
                Dir::default(),
                globe_spec,
                Some(globe_entity),
            ))
            .with(cell_dweller_visual)
            .with(Spatial::root())
            .build();

        let mesh_repo_ptr = Arc::new(Mutex::new(mesh_repo));
        let render_sys = render::System::new(
            factory,
            render_sys_encoder_channel,
            window.output_color.clone(),
            window.output_stencil.clone(),
            projection.clone(),
            &log,
            mesh_repo_ptr.clone(),
        );
        // Event channel for camera system
        let (camera_input_sender, camera_input_receiver) = mpsc::channel();
        let camera_update_sys = render::player_camera::System::new(camera_input_receiver);

        let mut planner = specs::Planner::new(world, 2);
        planner.add_system(movement_sys, "cd_movement", 100);
        planner.add_system(mining_sys, "cd_mining", 100);
        planner.add_system(render_sys, "render", 50);
        planner.add_system(camera_update_sys, "camera_update", 50);

        App {
            t: 0.0,
            log: log,
            planner: planner,
            encoder_channel: device_encoder_channel,
            movement_input_sender: movement_input_sender,
            mining_input_sender: mining_input_sender,
            projection: projection,
            camera_input_sender: camera_input_sender,
            factory: factory.clone(),
            output_color: window.output_color.clone(),
            output_stencil: window.output_stencil.clone(),
            mesh_repo: mesh_repo_ptr,
        }
    }

    pub fn run(&mut self, mut window: &mut PistonWindow) {
        use piston::input::*;
        use piston::event_loop::Events;

        info!(self.log, "Starting event loop");

        let mut events = window.events();
        while let Some(e) = events.next(window) {
            if let Some(r) = e.render_args() {
                self.render(&r, &mut window);
            }

            if e.resize_args().is_some() {
                let mut projection = self.projection.lock().unwrap();
                *projection = get_projection(window);
            }

            if let Some(u) = e.update_args() {
                self.update(&u);
            }

            use piston::input::keyboard::Key;
            use cell_dweller::{ MovementEvent, MiningEvent };
            if let Some(Button::Keyboard(key)) = e.press_args() {
                match key {
                    Key::I => self.movement_input_sender.send(MovementEvent::StepForward(true)).unwrap(),
                    Key::K => self.movement_input_sender.send(MovementEvent::StepBackward(true)).unwrap(),
                    Key::J => self.movement_input_sender.send(MovementEvent::TurnLeft(true)).unwrap(),
                    Key::L => self.movement_input_sender.send(MovementEvent::TurnRight(true)).unwrap(),
                    Key::U => self.mining_input_sender.send(MiningEvent::PickUp(true)).unwrap(),
                    _ => (),
                }
            }
            if let Some(Button::Keyboard(key)) = e.release_args() {
                match key {
                    Key::I => self.movement_input_sender.send(MovementEvent::StepForward(false)).unwrap(),
                    Key::K => self.movement_input_sender.send(MovementEvent::StepBackward(false)).unwrap(),
                    Key::J => self.movement_input_sender.send(MovementEvent::TurnLeft(false)).unwrap(),
                    Key::L => self.movement_input_sender.send(MovementEvent::TurnRight(false)).unwrap(),
                    Key::U => self.mining_input_sender.send(MiningEvent::PickUp(false)).unwrap(),
                    _ => (),
                }
            }

            self.camera_input_sender.send(e);
        }

        info!(self.log, "Quitting");
    }

    fn render(&mut self, _args: &RenderArgs, window: &mut PistonWindow) {
        // TODO: Systems are currently run on the main thread,
        // so we need to `try_recv` to avoid deadlock.
        // This is only because I don't want to burn CPU, and I've yet
        // to get around to frame/update rate limiting, so I'm
        // relying on Piston's for now.
        use std::sync::mpsc::TryRecvError;
        let mut encoder = match self.encoder_channel.receiver.try_recv() {
            Ok(encoder) => encoder,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => panic!("Render system hung up. That wasn't supposed to happen!"),
        };

        // TODO: what's make_current actually necessary for?
        // Do I even need to do this? (Ripped off `draw_3d`.)
        use piston::window::OpenGLWindow;
        window.window.make_current();

        encoder.flush(&mut window.device);

        self.encoder_channel.sender.send(encoder).unwrap();
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;
        self.planner.dispatch(args.dt);

        self.realize_proto_meshes();
    }

    // This whole thing is a horrible hack around
    // not being able to create GL resource factories
    // on other threads. It's acting as a proof that
    // I can make this work, at which point I should gut
    // the whole disgusting thing and find a better way
    // to work around the root problem.
    fn realize_proto_meshes(&mut self) {
        // NOTE: it is essential that we lock the world first.
        // Otherwise we could dead-lock against, e.g., the render
        // system while it's trying to lock the mesh repository.
        let world = self.planner.mut_world();
        let mut mesh_repo = self.mesh_repo.lock().unwrap();
        let mut visuals = world.write::<Visual>();
        use specs::Join;
        for visual in (&mut visuals).iter() {
            // Even if there's a realized mesh already, the presence of
            // a proto-mesh indicates we need to realize again.
            // (We clear out the proto-mesh when we realize it.)
            let needs_to_be_realized = visual.proto_mesh.is_some();
            if !needs_to_be_realized {
                continue;
            }
            let proto_mesh = visual.proto_mesh.clone().expect("Just ensured this above...");
            let mesh = Mesh::new(
                &mut self.factory,
                proto_mesh.vertexes.clone(),
                proto_mesh.indexes.clone(),
                self.output_color.clone(),
                self.output_stencil.clone(),
            );
            if let Some(existing_mesh_handle) = visual.mesh_handle() {
                // We're replacing an existing mesh that got dirty.
                mesh_repo.replace_mesh(existing_mesh_handle, mesh);
            } else {
                // We're realizing this mesh for the first time.
                let mesh_handle = mesh_repo.add_mesh(mesh);
                visual.set_mesh_handle(mesh_handle);
            }
            visual.proto_mesh = None;
        }
    }
}

impl<'a> App {
    pub fn planner(&'a mut self) -> &'a mut specs::Planner<TimeDelta> {
        &mut self.planner
    }
}
