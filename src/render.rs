use crate::{cosmic_edit::ReadOnly, prelude::*};
use crate::{cosmic_edit::*, CosmicWidgetSize};
use bevy::ecs::query::QueryData;
use bevy::ecs::system::SystemParam;
use bevy::render::render_resource::Extent3d;
use cosmic_text::{Color, Edit};
use image::{imageops::FilterType, GenericImageView};

/// System set for cosmic text rendering systems. Runs in [`PostUpdate`]
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RenderSet;

pub(crate) struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<SwashCache>() {
            app.insert_resource(SwashCache::default());
        } else {
            debug!("Skipping inserting `SwashCache` resource");
        }
        app.add_systems(Update, blink_cursor)
            .add_systems(PostUpdate, (render_texture,).in_set(RenderSet));
    }
}

pub(crate) fn blink_cursor(mut q: Query<&mut CosmicEditor, Without<ReadOnly>>, time: Res<Time>) {
    for mut e in q.iter_mut() {
        e.cursor_timer.tick(time.delta());
        if e.cursor_timer.just_finished() {
            e.cursor_visible = !e.cursor_visible;
            e.set_redraw(true);
        }
    }
}

fn draw_pixel(buffer: &mut [u8], width: i32, height: i32, x: i32, y: i32, color: Color) {
    let a_a = color.a() as u32;
    if a_a == 0 {
        // Do not draw if alpha is zero
        return;
    }

    if y < 0 || y >= height {
        // Skip if y out of bounds
        return;
    }

    if x < 0 || x >= width {
        // Skip if x out of bounds
        return;
    }

    let offset = (y as usize * width as usize + x as usize) * 4;

    let bg = bevy::prelude::Color::srgba_u8(
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    );

    // TODO: if alpha is 100% or bg is empty skip blending

    let fg = Srgba::rgba_u8(color.r(), color.g(), color.b(), color.a());

    let premul = (fg * fg.alpha).with_alpha(color.a() as f32 / 255.0);

    let out = premul + (bg.to_srgba() * (1.0 - fg.alpha));

    buffer[offset] = (out.red * 255.0) as u8;
    buffer[offset + 1] = (out.green * 255.0) as u8;
    buffer[offset + 2] = (out.blue * 255.0) as u8;
    buffer[offset + 3] = (out.alpha * 255.0) as u8;
}

pub(crate) struct WidgetBufferCoordTransformation {
    /// Padding between the top of the render target and the
    /// top of the buffer
    top_padding: f32,
}

impl WidgetBufferCoordTransformation {
    pub fn new(
        vertical_align: VerticalAlign,
        render_target_height: f32,
        buffer_height: f32,
    ) -> Self {
        let top_padding = match vertical_align {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Bottom => (render_target_height - buffer_height).max(0.0),
            VerticalAlign::Center => ((render_target_height - buffer_height) / 2.0).max(0.0),
        };
        // debug!(?top_padding, ?render_target_height, ?buffer_height);
        Self { top_padding }
    }

    /// If you have the buffer coord, e.g. buffer is rendering
    pub fn buffer_to_widget(&self, buffer: Vec2) -> Vec2 {
        Vec2::new(buffer.x, buffer.y + self.top_padding)
    }

    /// Ifyou have the relative widget coord, e.g. mouse input
    pub fn widget_to_buffer(&self, widget: Vec2) -> Vec2 {
        Vec2::new(widget.x, widget.y - self.top_padding)
    }
}

/// Renders to the [CosmicRenderOutput]
fn render_texture(
    mut query: Query<(
        Option<&mut CosmicEditor>,
        &mut CosmicEditBuffer,
        &DefaultAttrs,
        &CosmicBackgroundImage,
        &CosmicBackgroundColor,
        &CursorColor,
        &SelectionColor,
        Option<&SelectedTextColor>,
        &CosmicRenderOutput,
        CosmicWidgetSize,
        Option<&ReadOnly>,
        &CosmicTextAlign,
        &CosmicWrap,
    )>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut images: ResMut<Assets<Image>>,
    mut swash_cache_state: ResMut<SwashCache>,
) {
    for (
        editor,
        mut buffer,
        attrs,
        background_image,
        fill_color,
        cursor_color,
        selection_color,
        selected_text_color_option,
        canvas,
        size,
        readonly_opt,
        text_align,
        wrap,
    ) in query.iter_mut()
    {
        let Ok(render_target_size) = size.logical_size() else {
            continue;
        };

        // avoids a panic
        if render_target_size.x == 0. || render_target_size.y == 0. {
            debug!(
                message = "Size of buffer is zero, skipping",
                // once = "This log only appears once"
            );
            continue;
        }

        // Draw background
        let mut pixels = vec![0; render_target_size.x as usize * render_target_size.y as usize * 4];
        if let Some(bg_image) = background_image.0.clone() {
            if let Some(image) = images.get(&bg_image) {
                let mut dynamic_image = image.clone().try_into_dynamic().unwrap();
                if image.size() != render_target_size.as_uvec2() {
                    dynamic_image = dynamic_image.resize_to_fill(
                        render_target_size.x as u32,
                        render_target_size.y as u32,
                        FilterType::Triangle,
                    );
                }
                for (i, (_, _, rgba)) in dynamic_image.pixels().enumerate() {
                    if let Some(p) = pixels.get_mut(i * 4..(i + 1) * 4) {
                        p[0] = rgba[0];
                        p[1] = rgba[1];
                        p[2] = rgba[2];
                        p[3] = rgba[3];
                    }
                }
            }
        } else {
            let bg = fill_color.0.to_cosmic();
            for pixel in pixels.chunks_exact_mut(4) {
                pixel[0] = bg.r(); // Red component
                pixel[1] = bg.g(); // Green component
                pixel[2] = bg.b(); // Blue component
                pixel[3] = bg.a(); // Alpha component
            }
        }

        let font_color = attrs
            .0
            .color_opt
            .unwrap_or(cosmic_text::Color::rgb(0, 0, 0));

        // compute alignment and y-offset
        let buffer_height = buffer.height();
        let render_target_height = render_target_size.y;
        let transformation = WidgetBufferCoordTransformation::new(
            text_align.vertical,
            render_target_height,
            buffer_height,
        );

        let draw_closure = |x, y, w, h, color| {
            for row in 0..h as i32 {
                for col in 0..w as i32 {
                    let buffer_coord = IVec2::new(x + col, y + row);
                    let widget_coord = transformation
                        .buffer_to_widget(buffer_coord.as_vec2())
                        .as_ivec2();
                    draw_pixel(
                        &mut pixels,
                        render_target_size.x as i32,
                        render_target_size.y as i32,
                        widget_coord.x,
                        widget_coord.y,
                        color,
                    );
                }
            }
        };

        let mut update_buffer_size = |buffer: &mut Buffer| {
            buffer.set_size(
                &mut font_system.0,
                Some(match wrap {
                    CosmicWrap::Wrap => render_target_size.x,
                    CosmicWrap::InfiniteLine => f32::MAX,
                }),
                Some(render_target_size.y),
            );
        };
        let update_buffer_horizontal_alignment = |buffer: &mut Buffer| {
            if let Some(alignment) = text_align.horizontal {
                for line in &mut buffer.lines {
                    line.set_align(Some(alignment.into()));
                }
            }
        };

        // Draw glyphs
        if let Some(mut editor) = editor {
            // todo: optimizations (see below comments)
            editor.set_redraw(true);
            if !editor.redraw() {
                continue;
            }

            let cursor_color = cursor_color.0;
            let cursor_opacity = if editor.cursor_visible && readonly_opt.is_none() {
                cursor_color.alpha()
            } else {
                0.
            };

            let cursor_color = cursor_color.with_alpha(cursor_opacity).to_cosmic();

            let selection_color = selection_color.0.to_cosmic();

            let selected_text_color = selected_text_color_option
                .map(|selected_text_color| selected_text_color.0.to_cosmic())
                .unwrap_or(font_color);

            editor.with_buffer_mut(update_buffer_size);
            editor.with_buffer_mut(update_buffer_horizontal_alignment);

            editor.with_buffer_mut(|buffer| buffer.shape_until_scroll(&mut font_system.0, false));

            editor.draw(
                &mut font_system.0,
                &mut swash_cache_state.0,
                font_color,
                cursor_color,
                selection_color,
                selected_text_color,
                draw_closure,
            );

            // TODO: Performance optimization, read all possible render-input
            // changes and only redraw if necessary
            // editor.set_redraw(false);
        } else {
            // todo: performance optimizations (see comments above/below)
            buffer.set_redraw(true);
            if !buffer.redraw() {
                continue;
            }

            update_buffer_size(&mut buffer);
            update_buffer_horizontal_alignment(&mut buffer);

            buffer.shape_until_scroll(&mut font_system.0, false);

            buffer.draw(
                &mut font_system.0,
                &mut swash_cache_state.0,
                font_color,
                draw_closure,
            );

            // TODO: Performance optimization, read all possible render-input
            // changes and only redraw if necessary
            // buffer.set_redraw(false);
        }

        if let Some(prev_image) = images.get_mut(&canvas.0) {
            prev_image.data.clear();
            // Updates the stored asset image with the computed pixels
            prev_image.data.extend_from_slice(pixels.as_slice());
            prev_image.resize(Extent3d {
                width: render_target_size.x as u32,
                height: render_target_size.y as u32,
                depth_or_array_layers: 1,
            });
        }
    }
}
