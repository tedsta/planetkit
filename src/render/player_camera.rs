use std::sync::mpsc;

use piston::input;
use specs;
use specs::Join;

use ::types::*;

#[derive(Default)]
pub struct ClientPlayer;
impl specs::Component for ClientPlayer {
    type Storage = specs::NullStorage<ClientPlayer>;
}

// Camera update system
pub struct System {
    camera_input_receiver: mpsc::Receiver<input::Input>,
}

impl System {
    pub fn new(input_receiver: mpsc::Receiver<input::Input>) -> System {
        System {
            camera_input_receiver: input_receiver,
        }
    }
}

impl specs::System<TimeDelta> for System {
    fn run(&mut self, arg: specs::RunArg, _: TimeDelta) {
        use na::{ Norm, Rotate };
        use ::Spatial;
        use ::types::Vec3;

        let (client_players, spatials, mut camera) = arg.fetch(|w| {
            (w.read::<ClientPlayer>(), w.read::<Spatial>(), w.write_resource::<Camera>())
        });
        // Handle incoming keyboard/mouse events for the PlayerCamera
        while let Ok(_) = self.camera_input_receiver.try_recv() {
            //camera.event(&e);
        }
        // Update the PlayerCamera's target position
        for (i, (_, s)) in (&client_players.check(), &spatials).iter().enumerate() {
            let player_pos = s.transform.translation;

            let up = player_pos.normalize(); // TODO player_pos - planet_pos
            let forward = s.transform.rotation.rotate(&Vec3::new(0.0, 0.0, 1.0));
            let left = s.transform.rotation.rotate(&Vec3::new(1.0, 0.0, 0.0));

            let target = player_pos + forward * 0.5;
            let cam_pos = player_pos + up * 0.15 - forward * 0.15 - left * 0.05;

            camera.position = [cam_pos.x, cam_pos.y, cam_pos.z];
            camera.up = [up.x, up.y, up.z];
            camera.look_at([target.x, target.y, target.z]);

            // Ensure there isn't more than one client player
            assert!(i == 0, "There is more than one client player!");
        }
    }
}
