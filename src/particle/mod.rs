use bevy::prelude::*;
pub struct ParticlePlugin;
impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct Particle {}

fn setup() {}
