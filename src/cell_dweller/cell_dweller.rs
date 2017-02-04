use specs;

use ::types::*;
use globe::{ CellPos, Dir, Spec };
use ::movement::*;

pub struct CellDweller {
    // TODO: make these private and use guts trait pattern to expose them internally.
    // TODO: is this guts pattern worth a separate macro crate of its own?
    pub pos: CellPos,
    pub dir: Dir,
    pub last_turn_bias: TurnDir,
    pub yaw: f64,
    pub pitch: f64,
    // Most `CellDweller`s will also be `Spatial`s. Track whether the
    // computed real-space transform has been updated since the globe-space
    // transform was modified so we know when the former is dirty.
    is_real_space_transform_dirty: bool,
    pub globe_spec: Spec,
    pub seconds_between_moves: TimeDelta,
    pub seconds_until_next_move: TimeDelta,
    pub seconds_between_turns: TimeDelta,
    pub seconds_until_next_turn: TimeDelta,
    pub seconds_until_next_fall: TimeDelta,
    pub globe_entity: Option<specs::Entity>,
}

impl CellDweller {
    pub fn new(pos: CellPos, dir: Dir, globe_spec: Spec, globe_entity: Option<specs::Entity>) -> CellDweller {
        CellDweller {
            pos: pos,
            dir: dir,
            last_turn_bias: TurnDir::Right,
            yaw: 0.0,
            pitch: 0.0,
            is_real_space_transform_dirty: true,
            globe_spec: globe_spec,
            // TODO: accept as parameter
            seconds_between_moves: 0.1,
            seconds_until_next_move: 0.0,
            // TODO: accept as parameter
            seconds_between_turns: 0.2,
            seconds_until_next_turn: 0.0,
            seconds_until_next_fall: 0.0,
            globe_entity: globe_entity,
        }
    }

    pub fn pos(&self) -> CellPos {
        self.pos
    }

    pub fn set_cell_pos(
        &mut self,
        new_pos: CellPos,
    ) {
        self.pos = new_pos;
        self.is_real_space_transform_dirty = true;
    }

    pub fn set_cell_transform(
        &mut self,
        new_pos: CellPos,
        new_dir: Dir,
        new_last_turn_bias: TurnDir,
    ) {
        self.pos = new_pos;
        self.dir = new_dir;
        self.last_turn_bias = new_last_turn_bias;
        self.is_real_space_transform_dirty = true;
    }

    pub fn dir(&self) -> Dir {
        self.dir
    }

    pub fn pan(&mut self, pan: f64) {
        use std::f64;
        if pan == 0.0 {
            return;
        }
        let turn_dir = if pan > 0.0 { TurnDir::Left } else { TurnDir::Right };
        let angle_between_edges =
            if is_pentagon(&self.pos, self.globe_spec.root_resolution) {
                2.0 * f64::consts::PI / 5.0
            } else {
                2.0 * f64::consts::PI / 6.0
            };
        let half_angle = angle_between_edges / 2.0;

        self.yaw += pan;
        while f64::abs(self.yaw) > half_angle {
            match turn_dir {
                TurnDir::Left => {
                    self.turn(turn_dir);
                    self.yaw -= angle_between_edges;
                },
                TurnDir::Right => {
                    self.turn(turn_dir);
                    self.yaw += angle_between_edges;
                },
            }
        }

        self.is_real_space_transform_dirty = true;
    }

    pub fn turn(&mut self, turn_dir: TurnDir) {
        turn_by_one_hex_edge(
            &mut self.pos,
            &mut self.dir,
            self.globe_spec.root_resolution,
            turn_dir,
        ).expect("This suggests a bug in `movement` code.");
        self.is_real_space_transform_dirty = true;
    }

    /// Calculate position in real-space.
    fn real_pos(&self) -> Pt3 {
        self.globe_spec.cell_bottom_center(self.pos)
    }

    fn real_transform(&self) -> Iso3 {
        use na::Rotation;

        let eye = self.real_pos();
        let next_pos = adjacent_pos_in_dir(self.pos, self.dir).unwrap(); // Look one cell ahead.
        let target = self.globe_spec.cell_bottom_center(next_pos);

        // Calculate up vector. Nalgebra will normalise this so we can
        // just use the eye position as a vector; it points up out from
        // the center of the world already!
        let up = eye.to_vector();
        let aim = Vec3::new(0.0, self.yaw, 0.0);

        let mut rotation = Rot3::new_observer_frame(&(target - eye), &up);
        rotation.prepend_rotation_mut(&aim);
        Iso3::from_rotation_matrix(eye.to_vector(), rotation)
    }

    pub fn is_real_space_transform_dirty(&self) -> bool {
        self.is_real_space_transform_dirty
    }

    // TODO: document responsibilities of caller.
    // TODO: return translation and orientation.
    pub fn get_real_transform_and_mark_as_clean(&mut self) -> Iso3 {
        self.is_real_space_transform_dirty = false;
        self.real_transform()
    }
}

impl specs::Component for CellDweller {
    type Storage = specs::HashMapStorage<CellDweller>;
}
