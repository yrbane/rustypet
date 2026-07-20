//! Machine à états du pet : animations, physique, spawns et enfants.
//! Voir `docs/reference/esheep-engine.md` §2, §4 et §5.

mod anim_state;

pub use anim_state::{AnimState, StepValues, interpolate, pick_frame, total_steps};
