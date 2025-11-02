use std::{cmp::Ordering, time::Duration};

use bevy::prelude::*;

use crate::{
    asset::LoadResource,
    callback::Callback,
    ui::{UiFont, UiFontSize},
};

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DialogueAssets>();
        app.load_resource::<DialogueAssets>();
        app.init_resource::<DialogueRes>();
        app.add_observer(handle_start_dialogue);
        app.add_systems(Update, do_dialogue);
    }
}

#[derive(Resource, Reflect, Asset, Clone)]
#[reflect(Resource)]
pub struct DialogueAssets {
    #[dependency]
    pub lyra_sad: Handle<Image>,
    #[dependency]
    pub lyra_happy: Handle<Image>,
    #[dependency]
    pub lyra_neutral: Handle<Image>,
    #[dependency]
    pub cruciera: Handle<Image>,
}

impl FromWorld for DialogueAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            lyra_sad: asset_server.load("dialogue-box-lyra-sad.png"),
            lyra_happy: asset_server.load("dialogue-box-lyra-happy.png"),
            lyra_neutral: asset_server.load("dialogue-box-lyra-neutral.png"),
            cruciera: asset_server.load("dialogue-box-cruciera.png"),
        }
    }
}

#[derive(Clone)]
pub struct DialogueEntry {
    pub text: String,
    pub image: Handle<Image>,
}

#[derive(Event, Clone)]
pub struct Dialogue {
    pub entries: Vec<DialogueEntry>,
    pub callback_entity: Option<Entity>,
    pub duration: Duration,
}

#[derive(Resource, Default)]
pub struct DialogueRes {
    dialogue: Option<Dialogue>,
    timer: Option<Timer>,
    index: usize,
}

#[derive(Component)]
pub struct DialogueBox;

#[derive(Component)]
pub struct DialogueText;

#[derive(Component)]
pub struct DialogueImage;

pub fn handle_start_dialogue(
    event: On<Dialogue>,
    mut commands: Commands,
    mut dialogue_res: ResMut<DialogueRes>,
    ui_font: Res<UiFont>,
) {
    if dialogue_res.dialogue.is_some() {
        warn!("Duplicate dialogue");
        return;
    }
    dialogue_res.dialogue = Some(event.clone());
    dialogue_res.timer = Some(Timer::new(event.duration, TimerMode::Repeating));
    dialogue_res.index = 0;

    let container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            display: Display::Flex,
            padding: UiRect::top(Val::Vh(5.)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        })
        .insert(DialogueBox)
        .id();

    let image = commands
        .spawn(Node {
            width: Val::Px(1280.),
            max_width: Val::Vw(80.),
            height: Val::Auto,
            aspect_ratio: Some(2775. / 630.), // FIXME: magic values!
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .insert(DialogueImage)
        .insert(ImageNode::new(event.entries[0].image.clone()))
        .insert(ChildOf(container))
        .id();

    commands
        .spawn(Node {
            margin: UiRect::new(
                Val::Percent(20.),
                Val::Percent(20.),
                Val::Percent(7.),
                Val::Percent(5.),
            ),
            ..default()
        })
        .insert(ui_font.text_font().with_font_size(UiFontSize::TEXT))
        .insert(TextLayout::new_with_justify(Justify::Center))
        .insert(Text::new(""))
        .insert(ChildOf(image))
        .insert(DialogueText);
}

#[allow(clippy::too_many_arguments)]
pub fn do_dialogue(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    dialogue_res: ResMut<DialogueRes>,
    dialogue_box: Single<Entity, With<DialogueBox>>,
    mut text: Single<&mut Text, With<DialogueText>>,
    mut image: Single<&mut ImageNode, With<DialogueImage>>,
) {
    let dialogue_res = dialogue_res.into_inner();
    if dialogue_res.timer.is_none() {
        return;
    }
    if dialogue_res.dialogue.is_none() {
        return;
    }
    let dialogue = dialogue_res.dialogue.as_ref().unwrap();
    let wanted_text = dialogue.entries[dialogue_res.index].text.as_str();

    if keys.any_just_pressed([KeyCode::Space, KeyCode::Enter])
        || mouse.just_pressed(MouseButton::Left)
    {
        match text.len().cmp(&wanted_text.len()) {
            Ordering::Less => {
                //if animating the text rn, display it fully
                **text = Text::new(wanted_text);
            }
            Ordering::Equal => {
                if dialogue_res.index + 1 >= dialogue.entries.len() {
                    if let Some(callback_entity) = dialogue.callback_entity {
                        commands.trigger(Callback {
                            entity: callback_entity,
                        });
                    }
                    dialogue_res.index = 0;
                    dialogue_res.dialogue = None;
                    dialogue_res.timer = None;

                    commands.entity(*dialogue_box).despawn();
                } else {
                    dialogue_res.index += 1;
                    **text = Text::new("");
                    image.image = dialogue.entries[dialogue_res.index].image.clone();
                }
            }
            _ => {}
        }
        return;
    }

    let timer = dialogue_res.timer.as_mut().unwrap();
    timer.tick(time.delta());
    // normal function call, animate text and then  update it
    if text.len() < wanted_text.len() && timer.just_finished() {
        **text = Text::new(&wanted_text[0..text.len() + 1]);
    }
}
