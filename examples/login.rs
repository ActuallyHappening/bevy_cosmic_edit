use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResolution},
};
use bevy_cosmic_edit::*;

#[derive(Component)]
struct SubmitButton;

#[derive(Component)]
struct UsernameTag;

#[derive(Component)]
struct PasswordTag;

#[derive(Component)]
struct DisplayTag;

fn setup(mut commands: Commands, mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window.single_mut();

    window.resolution = WindowResolution::new(330.0, 480.0);

    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(15.0)),
                width: Val::Px(330.0),

                ..default()
            },
            ..default()
        })
        .with_children(|root| {
            root.spawn(CosmicEditBundle {
                max_lines: CosmicMaxLines(1),
                metrics: CosmicMetrics {
                    scale_factor: window.scale_factor() as f32,
                    ..default()
                },
                ..default()
            })
            .insert(ButtonBundle {
                style: Style {
                    // Size and position of text box
                    width: Val::Px(300.),
                    height: Val::Px(50.),
                    margin: UiRect::all(Val::Px(15.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            })
            .insert(CosmicEditPlaceholderBundle {
                text_setter: PlaceholderText(CosmicText::OneStyle("Username".into())),
                attrs: PlaceholderAttrs(AttrsOwned::new(
                    Attrs::new().color(bevy_color_to_cosmic(Color::rgb_u8(128, 128, 128))),
                )),
            })
            .insert(UsernameTag);

            root.spawn(CosmicEditBundle {
                max_lines: CosmicMaxLines(1),
                metrics: CosmicMetrics {
                    scale_factor: window.scale_factor() as f32,
                    ..default()
                },
                ..default()
            })
            .insert(ButtonBundle {
                style: Style {
                    // Size and position of text box
                    width: Val::Px(300.),
                    height: Val::Px(50.),
                    margin: UiRect::all(Val::Px(15.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            })
            .insert(CosmicEditPlaceholderBundle {
                text_setter: PlaceholderText(CosmicText::OneStyle("Password".into())),
                attrs: PlaceholderAttrs(AttrsOwned::new(
                    Attrs::new().color(bevy_color_to_cosmic(Color::rgb_u8(128, 128, 128))),
                )),
            })
            .insert(PasswordTag)
            .insert(PasswordInput::default());

            root.spawn(CosmicEditBundle {
                max_lines: CosmicMaxLines(1),
                metrics: CosmicMetrics {
                    font_size: 25.0,
                    line_height: 25.0,
                    scale_factor: window.scale_factor() as f32,
                    ..default()
                },
                attrs: CosmicAttrs(AttrsOwned::new(
                    Attrs::new().color(bevy_color_to_cosmic(Color::WHITE)),
                )),
                text_setter: CosmicText::OneStyle("Submit".into()),
                fill_color: FillColor(Color::GREEN),
                ..default()
            })
            .insert(ButtonBundle {
                style: Style {
                    // Size and position of text box
                    width: Val::Px(150.),
                    height: Val::Px(50.),
                    margin: UiRect::all(Val::Px(15.0)),
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                border_color: Color::DARK_GREEN.into(),

                ..default()
            })
            .insert(SubmitButton)
            .insert(ReadOnly);

            root.spawn(CosmicEditBundle {
                metrics: CosmicMetrics {
                    scale_factor: window.scale_factor() as f32,
                    ..default()
                },
                ..default()
            })
            .insert(ButtonBundle {
                style: Style {
                    // Size and position of text box
                    width: Val::Px(300.),
                    height: Val::Px(100.),
                    margin: UiRect::all(Val::Px(15.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            })
            .insert(CosmicEditPlaceholderBundle {
                text_setter: PlaceholderText(CosmicText::OneStyle("Output".into())),
                attrs: PlaceholderAttrs(AttrsOwned::new(
                    Attrs::new().color(bevy_color_to_cosmic(Color::rgb_u8(128, 128, 128))),
                )),
            })
            .insert((ReadOnly, DisplayTag));
        });
}

fn bevy_color_to_cosmic(color: bevy::prelude::Color) -> CosmicColor {
    cosmic_text::Color::rgba(
        (color.r() * 255.) as u8,
        (color.g() * 255.) as u8,
        (color.b() * 255.) as u8,
        (color.a() * 255.) as u8,
    )
}

fn change_active_editor_ui(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, Entity),
        (
            Changed<Interaction>,
            (With<CosmicEditor>, Without<ReadOnly>),
        ),
    >,
) {
    for (interaction, entity) in interaction_query.iter_mut() {
        if let Interaction::Pressed = interaction {
            commands.insert_resource(Focus(Some(entity)));
        }
    }
}

fn print_changed_input(mut evr_type: EventReader<CosmicTextChanged>) {
    for ev in evr_type.iter() {
        println!("Changed: {}", ev.0 .1);
    }
}

fn submit_button(
    button_q: Query<&Interaction, With<SubmitButton>>,
    username_q: Query<
        &CosmicEditor,
        (With<UsernameTag>, Without<PasswordTag>, Without<DisplayTag>),
    >,
    password_q: Query<
        &CosmicEditor,
        (With<PasswordTag>, Without<UsernameTag>, Without<DisplayTag>),
    >,
    mut display_q: Query<
        (&mut CosmicEditor, &CosmicAttrs),
        (With<DisplayTag>, Without<UsernameTag>, Without<PasswordTag>),
    >,
    mut font_system: ResMut<CosmicFontSystem>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    for interaction in button_q.iter() {
        match interaction {
            Interaction::None => {}
            Interaction::Pressed => {
                let u = username_q.single();
                let p = password_q.single();
                let (mut d, attrs) = display_q.single_mut();

                let text = format!(
                    "Submitted!\nUsername: {}\nPassword: {}\n",
                    u.get_text(),
                    p.get_text()
                );

                d.set_text(
                    CosmicText::OneStyle(text),
                    attrs.0.clone(),
                    &mut font_system.0,
                );
            }
            Interaction::Hovered => {
                window.single_mut().cursor.icon = CursorIcon::Hand;
            }
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CosmicEditPlugin {
            change_cursor: CursorConfig::Default,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, change_active_editor_ui)
        .add_systems(Update, (print_changed_input, submit_button))
        .run();
}
