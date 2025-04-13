use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use super::entity::FixedEntityBundle;

pub struct MirrorPlugin;
impl Plugin for MirrorPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_int_cell_for_layer::<MirrorBundle>("Terrain", 16);
    }
}

#[derive(Default, Component)]
pub struct Mirror;

/// Bundle for Mirror
#[derive(Bundle, Default, LdtkIntCell)]
pub struct MirrorBundle {
    #[from_int_grid_cell]
    fixed_entity_bundle: FixedEntityBundle,
    mirror: Mirror,
}
