use std::time::Instant;

use egui::plot::{Plot, Value, Values};
use image::{
    imageops::FilterType::{Gaussian, Lanczos3},
    Pixels, RgbaImage,
};
use log::{debug, info};
use notan::{
    egui::{self, plot::Points, *},
    prelude::Graphics,
};

use crate::{
    image_editing::ImageOperation,
    update,
    utils::{
        disp_col, disp_col_norm, highlight_bleed, highlight_semitrans, send_extended_info,
        ImageExt, OculanteState, PaintStroke,
    },
};
pub trait EguiExt {
    fn label_i(&mut self, _text: &str) -> Response {
        unimplemented!()
    }
}

impl EguiExt for Ui {
    /// Draw a justified icon from a string starting with an emoji
    fn label_i(&mut self, text: &str) -> Response {
        let icon = text.chars().filter(|c| !c.is_ascii()).collect::<String>();
        let description = text.chars().filter(|c| c.is_ascii()).collect::<String>();
        self.with_layout(egui::Layout::right_to_left(), |ui| {
            // self.horizontal(|ui| {
            ui.add_sized(
                egui::Vec2::new(28., ui.available_height()),
                egui::Label::new(RichText::new(icon).color(ui.style().visuals.selection.bg_fill)),
            );
            ui.label(
                RichText::new(description).color(ui.style().visuals.noninteractive().text_color()),
            );
        })
        .response
    }
}

pub fn info_ui(ctx: &Context, state: &mut OculanteState, gfx: &mut Graphics) {
    if let Some(img) = &state.current_image {
        if let Some(p) = img.get_pixel_checked(
            state.cursor_relative.x as u32,
            state.cursor_relative.y as u32,
        ) {
            state.sampled_color = [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32];
        }
    }

    egui::SidePanel::left("side_panel").show(&ctx, |ui| {
        if let Some(texture) = &state.current_texture {
            // texture.
            let tex_id = gfx.egui_register_texture(&texture);

            // width of image widget
            let desired_width = ui.available_width();

            let scale = (desired_width / 8.) / texture.size().0;
            let img_size = egui::Vec2::new(desired_width, desired_width);

            let uv_center = (
                state.cursor_relative.x / state.image_dimension.0 as f32,
                (state.cursor_relative.y / state.image_dimension.1 as f32),
            );

            egui::Grid::new("info").show(ui, |ui| {
                ui.label_i("⬜ Size");

                ui.label(
                    RichText::new(format!(
                        "{}x{}",
                        state.image_dimension.0, state.image_dimension.1
                    ))
                    .monospace(),
                );
                ui.end_row();

                if let Some(path) = &state.current_path {
                    ui.label_i("🖻 File");
                    ui.label(
                        RichText::new(format!(
                            "{}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ))
                        .monospace(),
                    )
                    .on_hover_text(format!("{}", path.display()));
                    ui.end_row();
                }

                ui.label_i("🌗 RGBA");
                ui.label(
                    RichText::new(format!("{}", disp_col(state.sampled_color)))
                        .monospace()
                        .background_color(Color32::from_rgba_unmultiplied(255, 255, 255, 6)),
                );
                ui.end_row();

                ui.label_i("🌗 RGBA");
                ui.label(
                    RichText::new(format!("{}", disp_col_norm(state.sampled_color, 255.)))
                        .monospace()
                        .background_color(Color32::from_rgba_unmultiplied(255, 255, 255, 6)),
                );
                ui.end_row();

                ui.label_i("⊞ Pos");
                ui.label(
                    RichText::new(format!(
                        "{:.0},{:.0}",
                        state.cursor_relative.x, state.cursor_relative.y
                    ))
                    .monospace()
                    .background_color(Color32::from_rgba_unmultiplied(255, 255, 255, 6)),
                );
                ui.end_row();

                ui.label_i(" UV");
                ui.label(
                    RichText::new(format!("{:.3},{:.3}", uv_center.0, 1.0 - uv_center.1))
                        .monospace()
                        .background_color(Color32::from_rgba_unmultiplied(255, 255, 255, 6)),
                );
                ui.end_row();
            });

            // make sure aspect ratio is compensated for the square preview
            let ratio = texture.size().0 / texture.size().1;
            let uv_size = (scale, scale * ratio);
            let x = ui
                .add(
                    egui::Image::new(tex_id, img_size).uv(egui::Rect::from_x_y_ranges(
                        uv_center.0 - uv_size.0..=uv_center.0 + uv_size.0,
                        uv_center.1 - uv_size.1..=uv_center.1 + uv_size.1,
                    )), // .bg_fill(egui::Color32::RED),
                )
                .rect;

            let stroke_color = Color32::from_white_alpha(240);
            let bg_color = Color32::BLACK.linear_multiply(0.5);
            ui.painter_at(x).line_segment(
                [x.center_bottom(), x.center_top()],
                Stroke::new(4., bg_color),
            );
            ui.painter_at(x).line_segment(
                [x.left_center(), x.right_center()],
                Stroke::new(4., bg_color),
            );
            ui.painter_at(x).line_segment(
                [x.center_bottom(), x.center_top()],
                Stroke::new(1., stroke_color),
            );
            ui.painter_at(x).line_segment(
                [x.left_center(), x.right_center()],
                Stroke::new(1., stroke_color),
            );
            // ui.image(tex_id, img_size);
        }

        ui.vertical_centered_justified(|ui| {
            if let Some(img) = &state.current_image {
                if ui
                    .button("Show alpha bleed")
                    .on_hover_text("Highlight pixels with zero alpha and color information")
                    .clicked()
                {
                    state.current_texture = highlight_bleed(img).to_texture(gfx);
                }
                if ui
                    .button("Show semi-transparent pixels")
                    .on_hover_text(
                        "Highlight pixels that are neither fully opaque nor fully transparent",
                    )
                    .clicked()
                {
                    state.current_texture = highlight_semitrans(img).to_texture(gfx);
                }
                if ui.button("Reset image").clicked() {
                    state.current_texture = img.to_texture(gfx);
                }

                ui.add(egui::Slider::new(&mut state.tiling, 1..=10).text("Image tiling"));
            }
        });

        advanced_ui(ui, state);
    });
}

pub fn settings_ui(ctx: &Context, state: &mut OculanteState) {
    if state.settings_enabled {
        egui::Window::new("Settings")
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .default_width(400.)
            // .title_bar(false)
            .show(&ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    if ui.button("Check for updates").clicked() {
                        state.message = Some("Checking for updates...".into());
                        update::update(Some(state.message_channel.0.clone()));
                        state.settings_enabled = false;
                    }

                    if ui.button("Close").clicked() {
                        state.settings_enabled = false;
                    }
                });
            });
    }
}

pub fn advanced_ui(ui: &mut Ui, state: &mut OculanteState) {
    if let Some(info) = &state.image_info {
        egui::Grid::new("extended").show(ui, |ui| {
            ui.label(format!("Number of colors"));
            ui.label(format!("{}", info.num_colors));
            ui.end_row();

            ui.label("Fully transparent");
            ui.label(format!(
                "{:.2}%",
                (info.num_transparent_pixels as f32 / info.num_pixels as f32) * 100.
            ));
            ui.end_row();

            ui.label("Pixels");
            ui.label(format!("{}", info.num_pixels));
            ui.end_row();
        });

        let red_vals = Points::new(Values::from_values_iter(
            info.red_histogram
                .iter()
                .map(|(k, v)| Value::new(*k as f64, *v as f64)),
        ))
        // .fill(0.)
        .stems(0.0)
        .color(Color32::RED);

        let green_vals = Points::new(Values::from_values_iter(
            info.green_histogram
                .iter()
                .map(|(k, v)| Value::new(*k as f64, *v as f64)),
        ))
        // .fill(0.)
        .stems(0.0)
        .color(Color32::GREEN);

        let blue_vals = Points::new(Values::from_values_iter(
            info.blue_histogram
                .iter()
                .map(|(k, v)| Value::new(*k as f64, *v as f64)),
        ))
        // .fill(0.)
        .stems(0.0)
        .color(Color32::BLUE);

        Plot::new("histogram")
            .allow_zoom(false)
            .allow_drag(false)
            .show(ui, |plot_ui| {
                // plot_ui.line(grey_vals);
                plot_ui.points(red_vals);
                plot_ui.points(green_vals);
                plot_ui.points(blue_vals);
            });
    }
}

/// Everything related to image editing
pub fn edit_ui(ctx: &Context, state: &mut OculanteState, gfx: &mut Graphics) {
    egui::SidePanel::right("editing")
        .min_width(100.)
        .show(&ctx, |ui| {
            // A flag to indicate that the image needs to be rebuilt
            let mut image_changed = false;
            let mut pixels_changed = false;

            egui::Grid::new("editing").num_columns(2).show(ui, |ui| {
                ui.label_i("🔃 Rotation");
                ui.horizontal(|ui| {
                    if let Some(img) = &mut state.current_image {
                        let available_w_single_spacing =
                            ui.available_width() - ui.style().spacing.item_spacing.x;
                        if ui
                            .add_sized(
                                egui::vec2(available_w_single_spacing / 2., ui.available_height()),
                                egui::Button::new("⟳"),
                            )
                            .on_hover_text("Rotate 90 deg right")
                            .clicked()
                        {
                            *img = image::imageops::rotate90(img);
                            state.edit_state.resize = Default::default();

                            pixels_changed = true;
                        }
                        if ui
                            .add_sized(
                                egui::vec2(available_w_single_spacing / 2., ui.available_height()),
                                egui::Button::new("⟲"),
                            )
                            .on_hover_text("Rotate 90 deg left")
                            .clicked()
                        {
                            *img = image::imageops::rotate270(img);
                            state.edit_state.resize = Default::default();
                            pixels_changed = true;
                        }
                    }
                });
                ui.end_row();

                ui.label_i("⬌ Flipping");
                // ui.vertical_centered_justified(|ui| {
                ui.horizontal(|ui| {
                    if let Some(img) = &mut state.current_image {
                        let available_w_single_spacing =
                            ui.available_width() - ui.style().spacing.item_spacing.x;

                        if ui
                            .add_sized(
                                egui::vec2(available_w_single_spacing / 2., ui.available_height()),
                                egui::Button::new("Horizontal"),
                            )
                            .clicked()
                        {
                            *img = image::imageops::flip_horizontal(img);
                            pixels_changed = true;
                        }
                        if ui
                            .add_sized(
                                egui::vec2(available_w_single_spacing / 2., ui.available_height()),
                                egui::Button::new("Vertical"),
                            )
                            .clicked()
                        {
                            *img = image::imageops::flip_vertical(img);
                            pixels_changed = true;
                        }
                    }
                });
                ui.end_row();

                ui.label_i("✂ Crop");
                let available_w_single_spacing =
                    ui.available_width() - ui.style().spacing.item_spacing.x * 3.;
                ui.horizontal(|ui| {
                    let r1 = ui.add_sized(
                        egui::vec2(available_w_single_spacing / 4., ui.available_height()),
                        egui::DragValue::new(&mut state.edit_state.crop[0])
                            .speed(4.)
                            .clamp_range(0..=10000)
                            .prefix("⏴ "),
                    );
                    let r2 = ui.add_sized(
                        egui::vec2(available_w_single_spacing / 4., ui.available_height()),
                        egui::DragValue::new(&mut state.edit_state.crop[2])
                            .speed(4.)
                            .clamp_range(0..=10000)
                            .prefix("⏵ "),
                    );
                    let r3 = ui.add_sized(
                        egui::vec2(available_w_single_spacing / 4., ui.available_height()),
                        egui::DragValue::new(&mut state.edit_state.crop[1])
                            .speed(4.)
                            .clamp_range(0..=10000)
                            .prefix("⏶ "),
                    );
                    let r4 = ui.add_sized(
                        egui::vec2(available_w_single_spacing / 4., ui.available_height()),
                        egui::DragValue::new(&mut state.edit_state.crop[3])
                            .speed(4.)
                            .clamp_range(0..=10000)
                            .prefix("⏷ "),
                    );
                    // TODO rewrite with any
                    if r1.changed() || r2.changed() || r3.changed() || r4.changed() {
                        pixels_changed = true;
                    }
                });
                ui.end_row();

                let mut ops = [
                    ImageOperation::Brightness(0),
                    ImageOperation::Contrast(0),
                    ImageOperation::Desaturate(0),
                    ImageOperation::Blur(0),
                    ImageOperation::Invert,
                    ImageOperation::Mult([255, 255, 255]),
                    ImageOperation::Add([0, 0, 0]),
                    ImageOperation::Resize {
                        dimensions: state.image_dimension,
                        aspect: true,
                    },
                    ImageOperation::SwapRG,
                    ImageOperation::SwapBG,
                    ImageOperation::SwapRB,
                ];

                ui.label_i("➕ Filter");
                let available_w_single_spacing =
                    ui.available_width() - ui.style().spacing.item_spacing.x;

                egui::ComboBox::from_id_source("Imageops")
                    .selected_text("Select a filter to add...")
                    .width(available_w_single_spacing)
                    .show_ui(ui, |ui| {
                        for op in &mut ops {
                            if ui.selectable_label(false, format!("{}", op)).clicked() {
                                state.edit_state.edit_stack.push(*op);
                                pixels_changed = true;
                            }
                        }
                    });
                ui.end_row();

                let mut delete: Option<usize> = None;
                let mut swap: Option<(usize, usize)> = None;

                for (i, operation) in state.edit_state.edit_stack.iter_mut().enumerate() {
                    ui.label_i(&format!("{}", operation));
                    // let op draw itself and check for response
                    if operation.ui(ui).changed() {
                        pixels_changed = true;
                    }
                    // ui.horizontal(|ui| {
                    //     if ui.button("⏶").clicked() {
                    //         swap = Some(((i as i32 - 1).max(0) as usize, i));
                    //         pixels_changed = true;
                    //     }
                    //     if ui.button("⏷").clicked() {
                    //         swap = Some((i, i + 1));
                    //         pixels_changed = true;
                    //     }
                    //     if ui.button("⊗").clicked() {
                    //         delete = Some(i);
                    //         pixels_changed = true;
                    //     }
                    // });
                    ui.end_row();
                }
                if let Some(delete) = delete {
                    state.edit_state.edit_stack.remove(delete);
                }

                if let Some(swap) = swap {
                    if swap.1 < state.edit_state.edit_stack.len() {
                        state.edit_state.edit_stack.swap(swap.0, swap.1);
                    }
                }

                ui.label_i("🔁 Reset");
                ui.centered_and_justified(|ui| {
                    if ui.button("Reset all edits").clicked() {
                        state.edit_state = Default::default();
                        pixels_changed = true
                    }
                });
                ui.end_row();

                ui.label_i("❓ Compare");
                let available_w_single_spacing =
                    ui.available_width() - ui.style().spacing.item_spacing.x;
                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            egui::vec2(available_w_single_spacing / 2., ui.available_height()),
                            egui::Button::new("Unmodified"),
                        )
                        .clicked()
                    {
                        if let Some(img) = &state.current_image {
                            state.image_dimension = img.dimensions();
                            state.current_texture = img.to_texture(gfx);
                        }
                    }
                    if ui
                        .add_sized(
                            egui::vec2(available_w_single_spacing / 2., ui.available_height()),
                            egui::Button::new("Modified"),
                        )
                        .clicked()
                    {
                        pixels_changed = true;
                    }
                });
                ui.end_row();
            });

            ui.vertical_centered_justified(|ui| {
                if state.edit_state.painting {
                    if ui
                        .add(
                            egui::Button::new("Stop painting")
                                .fill(ui.style().visuals.selection.bg_fill),
                        )
                        .clicked()
                    {
                        state.edit_state.painting = false;
                    }
                } else {
                    if ui.button("🖊 Paint mode").clicked() {
                        state.edit_state.painting = true;
                    }
                }
            });

            if state.edit_state.painting {
                egui::Grid::new("paint").show(ui, |ui| {
                    ui.label("📜 Keep history");
                    ui.checkbox(&mut state.edit_state.non_destructive_painting, "")
                        .on_hover_text("Keep all paint history and edit it. Slower.");
                    ui.end_row();

                    if let Some(stroke) = state.edit_state.paint_strokes.last_mut() {
                        if stroke.is_empty() {
                            ui.label("Color");
                            ui.label("Fade");
                            ui.label("Flip");
                            ui.label("Width");
                            ui.label("Brush");
                            ui.end_row();

                            stroke_ui(stroke, &state.edit_state.brushes, ui, gfx);
                        }
                    }
                });

                if state
                    .edit_state
                    .paint_strokes
                    .iter()
                    .filter(|s| !s.is_empty())
                    .count()
                    != 0
                {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Strokes");
                        if ui.button("↩").clicked() {
                            let _ = state.edit_state.paint_strokes.pop();
                            let _ = state.edit_state.paint_strokes.pop();
                            pixels_changed = true;
                        }
                        if ui.button("Clear all").clicked() {
                            let _ = state.edit_state.paint_strokes.clear();
                            pixels_changed = true;
                        }
                    });

                    let mut delete_stroke: Option<usize> = None;

                    egui::ScrollArea::vertical()
                        .min_scrolled_height(64.)
                        .show(ui, |ui| {
                            let mut stroke_lost_highlight = false;
                            if ui
                                .vertical(|ui| {
                                    egui::Grid::new("stroke").show(ui, |ui| {
                                        ui.label("Color");
                                        ui.label("Fade");
                                        ui.label("Flip");
                                        ui.label("Width");
                                        ui.label("Brush");
                                        ui.label("Del");
                                        ui.end_row();

                                        for (i, stroke) in
                                            state.edit_state.paint_strokes.iter_mut().enumerate()
                                        {
                                            if stroke.is_empty() {
                                                continue;
                                            }

                                            let r = stroke_ui(
                                                stroke,
                                                &state.edit_state.brushes,
                                                ui,
                                                gfx,
                                            );
                                            if r.changed() {
                                                pixels_changed = true;
                                            }

                                            if r.hovered() {
                                                pixels_changed = true;
                                                stroke.highlight = true;
                                            } else {
                                                stroke.highlight = false;
                                                stroke_lost_highlight = true;
                                            }

                                            // safety catch to update brush highlights
                                            if r.clicked_elsewhere() {
                                                pixels_changed = true;
                                            }

                                            if ui.button("⊗").clicked() {
                                                delete_stroke = Some(i);
                                            }
                                            ui.end_row();
                                        }
                                    });
                                })
                                .response
                                .hovered()
                            {
                                // only update if this outer response is triggered, so we don't trigger it all the time
                                if stroke_lost_highlight {
                                    pixels_changed = true;
                                }
                            }
                        });
                    if let Some(stroke_to_delete) = delete_stroke {
                        state.edit_state.paint_strokes.remove(stroke_to_delete);
                        pixels_changed = true;
                    }
                }

                ui.end_row();

                // If we have no lines, create an empty one
                if state.edit_state.paint_strokes.is_empty() {
                    state.edit_state.paint_strokes.push(PaintStroke::new());
                }

                if let Some(current_stroke) = state.edit_state.paint_strokes.last_mut() {
                    // if state.mouse_delta.x > 0.0 {
                    if ctx.input().pointer.primary_down() && !state.pointer_over_ui {
                        debug!("PAINT");
                        // get pos in image
                        let p = state.cursor_relative;
                        current_stroke.points.push(Pos2::new(p.x, p.y));
                        pixels_changed = true;
                    } else if !current_stroke.is_empty() {
                        // clone last stroke to inherit settings
                        if let Some(last_stroke) = state.edit_state.paint_strokes.clone().last() {
                            state
                                .edit_state
                                .paint_strokes
                                .push(last_stroke.without_points());
                        }
                    }
                }
            }
            ui.end_row();

            if state.edit_state.result != Default::default() {
                ui.vertical_centered_justified(|ui| {
                    if ui
                        .button("⤵ Apply all edits")
                        .on_hover_text("Apply all edits to the image and reset edit controls")
                        .clicked()
                    {
                        if let Some(img) = &mut state.current_image {
                            *img = state.edit_state.result.clone();
                            state.edit_state = Default::default();
                            // state.image_dimension = img.dimensions();
                            pixels_changed = true;
                        }
                    }
                });
            }

            // Do the processing
            if pixels_changed {
                if let Some(img) = &mut state.current_image {
                    if state.edit_state.painting {
                        debug!("Num strokes {}", state.edit_state.paint_strokes.len());

                        // render previous strokes
                        if state
                            .edit_state
                            .paint_strokes
                            .iter()
                            .filter(|l| !l.points.is_empty())
                            .count()
                            > 1
                            && !state.edit_state.non_destructive_painting
                        {
                            // info!("initial strokes: {}", state.edit_state.paint_strokes.len());

                            // commit first line
                            if let Some(first_line) = state.edit_state.paint_strokes.first() {
                                first_line.render(img, &state.edit_state.brushes);
                                info!("Committed stroke");
                                state.edit_state.paint_strokes.remove(0);
                            }

                            // info!("strokes left: {}", state.edit_state.paint_strokes.len());
                        }
                    }

                    if state.edit_state.resize != img.dimensions()
                        && state.edit_state.resize != (0, 0)
                    {
                        state.edit_state.result = image::imageops::resize(
                            img,
                            state.edit_state.resize.0,
                            state.edit_state.resize.1,
                            Gaussian,
                        );
                    } else {
                        state.edit_state.result = img.clone();
                    }

                    // test if there is cropping, or copy original
                    if state.edit_state.crop != [0, 0, 0, 0] {
                        let sub_img = image::imageops::crop_imm(
                            &state.edit_state.result,
                            state.edit_state.crop[0].max(0) as u32,
                            state.edit_state.crop[1].max(0) as u32,
                            (img.width() as i32 - state.edit_state.crop[2]).max(0) as u32,
                            (img.height() as i32 - state.edit_state.crop[3]).max(0) as u32,
                        );
                        state.edit_state.result = sub_img.to_image();
                    }

                    if !state.edit_state.edit_stack.is_empty() {
                        let stamp = Instant::now();
                        for p in state.edit_state.result.pixels_mut() {
                            // convert pixel to f32 for processing, so we don't clamp
                            let mut float_pixel = image::Rgba([
                                p[0] as f32 / 255.,
                                p[1] as f32 / 255.,
                                p[2] as f32 / 255.,
                                p[3] as f32 / 255.,
                            ]);

                            for operation in &mut state.edit_state.edit_stack {
                                operation.process_pixel(&mut float_pixel);
                            }

                            // convert back
                            p[0] = (float_pixel[0].clamp(0.0, 1.0) * 255.) as u8;
                            p[1] = (float_pixel[1].clamp(0.0, 1.0) * 255.) as u8;
                            p[2] = (float_pixel[2].clamp(0.0, 1.0) * 255.) as u8;
                        }
                        info!("px elapsed{}", stamp.elapsed().as_secs_f32());

                        for operation in &mut state.edit_state.edit_stack {
                            operation.process_image(&mut state.edit_state.result);
                        }
                    }

                    // draw paint lines
                    // let stamp = std::time::Instant::now();
                    for stroke in &state.edit_state.paint_strokes {
                        stroke.render(&mut state.edit_state.result, &state.edit_state.brushes);
                    }
                    // debug!("Stroke rendering took {}s", stamp.elapsed().as_secs_f64());
                }

                // update the current texture with the edit state
                // let stamp = std::time::Instant::now();
                // state.current_texture = state.edit_state.result.to_texture(gfx);
                // info!("New tex took {}s", stamp.elapsed().as_secs_f64());

                // let stamp = std::time::Instant::now();
                if let Some(tex) = &mut state.current_texture {
                    if let Some(img) = &state.current_image {
                        if tex.width() as u32 == state.edit_state.result.width()
                            && state.edit_state.result.height() as u32 == img.height()
                        {
                            state.edit_state.result.update_texture(gfx, tex);
                        } else {
                            state.current_texture = state.edit_state.result.to_texture(gfx);
                        }
                    }
                }
                // info!("Upd tex took {}s", stamp.elapsed().as_secs_f64());
            }

            if state.edit_state.result != Default::default() {
                state.image_dimension = state.edit_state.result.dimensions();

                ui.vertical_centered_justified(|ui| {
                    let compatible_extensions = ["png", "jpg"];
                    if let Some(path) = &state.current_path {
                        if let Some(ext) = path.extension() {
                            if compatible_extensions
                                .contains(&ext.to_string_lossy().to_string().as_str())
                            {
                                if ui.button("💾 Overwrite").clicked() {
                                    let _ = state.edit_state.result.save(path);
                                }
                            } else {
                                if ui.button("💾 Save as png").clicked() {
                                    let _ =
                                        state.edit_state.result.save(path.with_extension("png"));
                                }
                            }
                        }

                        if ui
                            .button("⟳ Reload image")
                            .on_hover_text("Completely reload image, destroying all edits.")
                            .clicked()
                        {
                            state.is_loaded = false;
                            state.player.load(&path);
                        }
                    }
                });
            }

            if pixels_changed && state.info_enabled {
                state.image_info = None;
                send_extended_info(
                    &Some(state.edit_state.result.clone()),
                    &state.extended_info_channel,
                );
            }
        });
}

pub fn tooltip(r: Response, tooltip: &str, hotkey: &str, _ui: &mut Ui) -> Response {
    r.on_hover_ui(|ui| {
        ui.horizontal(|ui| {
            ui.label(tooltip);
            ui.label(
                RichText::new(hotkey)
                    .monospace()
                    .color(Color32::WHITE)
                    .background_color(ui.style().visuals.selection.bg_fill),
            );
        });
    })
}

pub fn unframed_button(text: impl Into<WidgetText>, ui: &mut Ui) -> Response {
    ui.add(egui::Button::new(text).frame(false))
}

pub fn stroke_ui(
    stroke: &mut PaintStroke,
    brushes: &Vec<RgbaImage>,
    ui: &mut Ui,
    gfx: &mut Graphics,
) -> Response {
    let mut combined_response = ui.color_edit_button_rgba_unmultiplied(&mut stroke.color);

    let r = ui
        .checkbox(&mut stroke.fade, "")
        .on_hover_text("Fade out the stroke over it's path");
    if r.changed() {
        combined_response.changed = true;
    }
    if r.hovered() {
        combined_response.hovered = true;
    }

    let r = ui
        .checkbox(&mut stroke.flip_random, "")
        .on_hover_text("Flip brush in X any Y randomly to make stroke less uniform");
    if r.changed() {
        combined_response.changed = true;
    }
    if r.hovered() {
        combined_response.hovered = true;
    }

    let r = ui.add(egui::DragValue::new(&mut stroke.width));
    if r.changed() {
        combined_response.changed = true;
    }
    if r.hovered() {
        combined_response.hovered = true;
    }

    ui.horizontal(|ui| {
        if let Some(notan_texture) = brushes[stroke.brush_index].to_texture_premult(gfx) {
            let texture_id = gfx.egui_register_texture(&notan_texture);
            ui.image(texture_id, egui::Vec2::splat(ui.available_height()));
        }

        let r = egui::ComboBox::from_id_source(format!("s {:?}", stroke.points))
            .selected_text(format!("Brush {}", stroke.brush_index))
            .show_ui(ui, |ui| {
                for (b_i, b) in brushes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if let Some(notan_texture) = b.to_texture_premult(gfx) {
                            let texture_id = gfx.egui_register_texture(&notan_texture);
                            ui.image(texture_id, egui::Vec2::splat(32.));
                        }

                        if ui
                            .selectable_value(
                                &mut stroke.brush_index,
                                b_i,
                                format!("Brush {}", b_i),
                            )
                            .clicked()
                        {
                            combined_response.changed = true
                        }
                    });
                }
            })
            .response;

        if r.hovered() {
            combined_response.hovered = true;
        }
    });

    if combined_response.hovered() {
        stroke.highlight = true;
    } else {
        stroke.highlight = false;
    }
    if combined_response.changed() {
        stroke.highlight = false;
    }
    combined_response
}
