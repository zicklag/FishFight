use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::*;
use bevy_fluent::Localization;

use crate::prelude::*;

pub struct DebugToolsPlugin;

impl Plugin for DebugToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(FrameTimeDiagnosticsPlugin)
            // .init_resource::<ShowNetworkVisualizer>()
            .init_resource::<ShowFameTimeDiagnostics>()
            .add_system(debug_tools_window)
            .add_system(frame_diagnostic_window);
    }
}

// #[derive(Deref, DerefMut, Default)]
// pub struct ShowNetworkVisualizer(pub bool);

// fn network_visualizer_window(
//     show: Res<ShowNetworkVisualizer>,
//     mut egui_context: ResMut<EguiContext>,
//     mut visualizer: ResMut<RenetServerVisualizer<200>>,
//     server: Res<RenetServer>,
// ) {
//     if **show {
//         visualizer.update(&server);
//         visualizer.show_window(egui_context.ctx_mut());
//     }
// }

#[derive(Resource, Default, Deref, DerefMut)]
struct ShowFameTimeDiagnostics(pub bool);

/// System that renders the debug tools window which can be toggled by pressing F12
fn debug_tools_window(
    // mut show_network_visualizer: ResMut<ShowNetworkVisualizer>,
    mut visible: Local<bool>,
    mut egui_context: ResMut<EguiContext>,
    // mut physics_debug_render: ResMut<PhysicsDebugRenderConfig>,
    mut show_frame_diagnostics: ResMut<ShowFameTimeDiagnostics>,
    localization: Res<Localization>,
    input: Res<Input<KeyCode>>,
    mut show_inspector: ResMut<WorldInspectorEnabled>,
) {
    let ctx = egui_context.ctx_mut();

    // Toggle debug window visibility
    if input.just_pressed(KeyCode::F12) {
        *visible = !*visible;
    }

    // // Shortcut to toggle collision shapes without having to use the menu
    // if input.just_pressed(KeyCode::F10) {
    //     physics_debug_render.enabled = !physics_debug_render.enabled;
    // }

    // Shortcut to toggle the inspector without having to use the menu
    if input.just_pressed(KeyCode::F9) {
        show_inspector.0 = !show_inspector.0;
    }

    // Shortcut to toggle frame diagnostics
    if input.just_pressed(KeyCode::F8) {
        **show_frame_diagnostics = !**show_frame_diagnostics;
    }

    // // Shortcut to toggle network visualizers
    // if input.just_pressed(KeyCode::F7) {
    //     **show_network_visualizer = !**show_network_visualizer;
    // }

    // Display debug tool window
    egui::Window::new(localization.get("debug-tools"))
        // ID is needed because title comes from localizaition which can change
        .id(egui::Id::new("debug_tools"))
        .open(&mut visible)
        .show(ctx, |ui| {
            // // Show collision shapes
            // ui.checkbox(
            //     &mut physics_debug_render.enabled,
            //     format!("{} ( F10 )", localization.get("show-collision-shapes")),
            // );

            // Show world inspector
            ui.checkbox(
                &mut show_inspector.0,
                format!("{} ( F9 )", localization.get("show-world-inspector")),
            );

            // Show frame time diagnostics
            ui.checkbox(
                &mut show_frame_diagnostics,
                format!("{} ( F9 )", localization.get("show-frame-time-diagnostics")),
            );

            // Show network visualizer
            // ui.checkbox(
            //     &mut show_network_visualizer,
            //     format!("{} ( F7 )", localization.get("show-network-visualizer")),
            // );
        });
}

struct FrameDiagState {
    min_fps: f64,
    max_fps: f64,
    min_frame_time: f64,
    max_frame_time: f64,
}

impl Default for FrameDiagState {
    fn default() -> Self {
        Self {
            min_fps: f64::MAX,
            max_fps: 0.0,
            min_frame_time: f64::MAX,
            max_frame_time: 0.0,
        }
    }
}

fn frame_diagnostic_window(
    mut state: Local<FrameDiagState>,
    mut egui_context: ResMut<EguiContext>,
    mut show: ResMut<ShowFameTimeDiagnostics>,
    diagnostics: Res<Diagnostics>,
    localization: Res<Localization>,
) {
    if **show {
        let ctx = egui_context.ctx_mut();

        egui::Window::new(&localization.get("frame-diagnostics"))
            .id(egui::Id::new("frame_diagnostics"))
            .default_width(500.0)
            .open(&mut show)
            .show(ctx, |ui| {
                if ui.button(&localization.get("reset-min-max")).clicked() {
                    *state = default();
                }

                let fps = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).unwrap();
                let fps_value = fps.value().unwrap();

                if fps_value < state.min_fps {
                    state.min_fps = fps_value;
                }
                if fps_value > state.max_fps {
                    state.max_fps = fps_value;
                }

                let frame_time = diagnostics
                    .get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
                    .unwrap();
                let frame_time_value = frame_time.value().unwrap();

                if frame_time_value < state.min_frame_time {
                    state.min_frame_time = frame_time_value;
                }
                if frame_time_value > state.max_frame_time {
                    state.max_frame_time = frame_time_value;
                }

                ui.monospace(&format!(
                    "{label:20}: {fps:4.0}{suffix:3} ( {min:4.0}{suffix:3}, {avg:4.0}{suffix:3}, {max:4.0}{suffix:3} )",
                    label = localization.get("frames-per-second"),
                    fps = fps_value,
                    suffix = fps.suffix,
                    min = state.min_fps,
                    avg = fps.average().unwrap(),
                    max = state.max_fps,
                ));
                ui.monospace(&format!(
                    "{label:20}: {fps:4.1}{suffix:3} ( {min:4.1}{suffix:3}, {avg:4.0}{suffix:3}, {max:4.1}{suffix:3} )",
                    label = localization.get("frame-time"),
                    fps = frame_time_value * 1000.0,
                    suffix = "ms",
                    min = state.min_frame_time * 1000.0,
                    avg = frame_time.average().unwrap() * 1000.0,
                    max = state.max_frame_time * 1000.0,
                ));
            });
    }
}
