use na;
use cam;

// Common types for all of PlanetKit.
//
// REVISIT: should some of these actually be `f32`
// for performance reasons? We definitely want
// `f64` for doing the non-realtime geometric
// manipulations, but entity positions etc. don't
// really need it.
pub type Vec2 = na::Vector2<f64>;
pub type Vec3 = na::Vector3<f64>;
pub type Pt2 = na::Point2<f64>;
pub type Pt3 = na::Point3<f64>;
pub type Rot3 = na::Rotation3<f64>;
pub type Iso3 = na::Isometry3<f64>;

pub type TimeDelta = f64;

pub type Mat4 = na::Matrix4<f64>;

pub type Camera = cam::Camera<f64>;
