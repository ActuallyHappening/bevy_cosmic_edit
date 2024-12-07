#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use crate::{
    cosmic_edit::ScrollEnabled,
    double_click::{ClickCount, ClickState},
    prelude::*,
    render_implementations::RelativeQuery,
};
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    input::mouse::{MouseScrollUnit, MouseWheel},
};
use cosmic_text::{Action, Edit, Motion, Selection};

pub mod clipboard;
pub mod hover;
pub mod keyboard;
// mod click;
pub mod click;
pub mod cursor_icon;
pub mod drag;
pub mod scroll;

/// System set for mouse and keyboard input events. Runs in [`PreUpdate`] and [`Update`]
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

pub(crate) struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, scroll::scroll.in_set(InputSet))
            .add_systems(
                Update,
                (
                    keyboard::kb_move_cursor,
                    keyboard::kb_input_text,
                    clipboard::kb_clipboard,
                    cursor_icon::update_cursor_hover_state,
                )
                    .chain()
                    .in_set(InputSet),
            )
            .add_event::<hover::TextHoverIn>()
            .register_type::<hover::TextHoverIn>()
            .add_event::<hover::TextHoverOut>()
            .add_event::<hover::TextHoverOut>();

        #[cfg(target_arch = "wasm32")]
        {
            let (tx, rx) = crossbeam_channel::bounded::<WasmPaste>(1);
            app.insert_resource(WasmPasteAsyncChannel { tx, rx })
                .add_systems(Update, poll_wasm_paste);
        }
    }
}

/// First variant is least important, last is most important
#[derive(Component, Default, Debug)]
#[require(ScrollEnabled)]
#[component(on_add = add_event_handlers)]
pub(crate) enum InputState {
    #[default]
    Idle,
    Hovering,
    Dragging {
        initial_buffer_coord: Vec2,
    },
}

fn add_event_handlers(
    mut world: DeferredWorld,
    targeted_entity: Entity,
    _component_id: ComponentId,
) {
    let mut observers = [
        Observer::new(click::handle_click),
        Observer::new(drag::handle_dragstart),
        Observer::new(drag::handle_dragend),
        Observer::new(drag::handle_drag),
        Observer::new(hover::handle_hover_start),
        Observer::new(hover::handle_hover_continue),
        Observer::new(hover::handle_hover_end),
        Observer::new(cancel),
    ];
    for observer in &mut observers {
        observer.watch_entity(targeted_entity);
    }
    world.commands().spawn_batch(observers);
}

// todo: avoid these warnings on ReadOnly
fn warn_no_editor_on_picking_event() {
    debug!(
        message = "Failed to get editor from picking event",
        note = "This is a false alarm for ReadOnly buffers",
        note = "Please only use the `InputState` component on entities with a `CosmicEditor` component",
        note = "`CosmicEditor` components should be automatically added to focussed `CosmicEditBuffer` entities",
    );
}

impl InputState {
    /// `Cancel` event handler
    pub fn cancel(&mut self) {
        trace!("Cancelling a pointer");
        *self = InputState::Idle;
    }
}

fn cancel(
    trigger: Trigger<Pointer<Cancel>>,
    mut editor: Query<&mut InputState, With<CosmicEditBuffer>>,
) {
    let Ok(mut input_state) = editor.get_mut(trigger.target) else {
        warn_no_editor_on_picking_event();
        return;
    };

    input_state.cancel();
}
