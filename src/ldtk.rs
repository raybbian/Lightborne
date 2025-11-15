use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_ecs_ldtk::{
    assets::LdtkProject,
    ldtk::{loaded_level::LoadedLevel, Level, Type},
    prelude::LdtkFields,
    LevelIid, LevelSelection,
};

use crate::game::setup::LevelAssets;

pub trait LevelExt {
    const START_FLAG_IDENT: &'static str;
    fn start_flag_pos(&self) -> Option<Vec2>;
    fn level_box(&self) -> Rect;
    fn level_id(&self) -> &String;
}

impl LevelExt for Level {
    const START_FLAG_IDENT: &'static str = "Start";

    fn start_flag_pos(&self) -> Option<Vec2> {
        let layers = self.layer_instances.as_ref().expect("Layers not found! (This is probably because you are using the \"Separate level files\" option.)");

        for layer in layers {
            if layer.layer_instance_type == Type::Entities {
                for entity in &layer.entity_instances {
                    if entity.identifier == Self::START_FLAG_IDENT {
                        // NOTE: flip y value because ldtk shenanigans
                        let (Some(x), Some(y)) = (entity.world_x, entity.world_y) else {
                            panic!("Start flag entity has no coordinates! (This is probably because your LDTK world is not in free layout mode.)");
                        };
                        return Some(Vec2::new(x as f32, -y as f32));
                    }
                }
            }
        }
        None
    }

    fn level_id(&self) -> &String {
        let level_id = self
            .get_string_field("LevelId")
            .expect("Levels should always have a level id!");

        if level_id.is_empty() {
            panic!("Level id for a level should not be empty!");
        }
        level_id
    }

    fn level_box(&self) -> Rect {
        Rect::new(
            self.world_x as f32,
            -self.world_y as f32,
            (self.world_x + self.px_wid) as f32,
            (-self.world_y - self.px_hei) as f32,
        )
    }
}

// pub trait LevelSelectionExt {
//     fn level<'p>(&self, project: &'p LdtkProject) -> Option<&'p Level>;
// }
//
// impl LevelSelectionExt for LevelSelection {
//     fn level<'p>(&self, project: &'p LdtkProject) -> Option<&'p Level> {
//         for (i, level) in project.json_data().levels.iter().enumerate() {
//             if self.is_match(&LevelIndices::in_root(i), level) {
//                 return Some(level);
//             }
//         }
//         None
//     }
// }

/// Use only in
#[derive(SystemParam)]
pub struct LdtkParam<'w> {
    ldtk_assets: Res<'w, Assets<LdtkProject>>,
    level_assets: Res<'w, LevelAssets>,
}

impl LdtkParam<'_> {
    pub fn project(&self) -> Option<&LdtkProject> {
        self.ldtk_assets.get(self.level_assets.ldtk_file.id())
    }
}

#[derive(SystemParam)]
pub struct LdtkLevelParam<'w> {
    pub ldtk_param: LdtkParam<'w>,
    pub level_selection: ResMut<'w, LevelSelection>,
}

impl LdtkLevelParam<'_> {
    pub fn level_by_iid<'s>(&'s self, iid: &LevelIid) -> Option<LoadedLevel<'s>> {
        self.ldtk_param
            .project()
            .and_then(|project| project.as_standalone().get_loaded_level_by_iid(iid.get()))
    }

    pub fn cur_level<'s>(&'s self) -> Option<LoadedLevel<'s>> {
        self.ldtk_param.project().and_then(|project| {
            project
                .as_standalone()
                .find_loaded_level_by_level_selection(&self.level_selection)
        })
    }

    pub fn cur_iid(&self) -> Option<LevelIid> {
        self.cur_level()
            .map(|level| LevelIid::new(level.raw().iid.as_str()))
    }
}
