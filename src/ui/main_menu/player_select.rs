use crate::loading::PlayerInputCollector;
use bones_lib::prelude::{key, Key, KeyError};

use super::*;

#[derive(Resource, Default)]
pub struct PlayerSelectState {
    pub slots: [PlayerSlot; MAX_PLAYERS],
}

#[derive(Default)]
pub struct PlayerSlot {
    pub active: bool,
    pub confirmed: bool,
    pub selected_player: bones::Handle<PlayerMeta>,
}

#[derive(SystemParam)]
pub struct PlayerSelectMenu<'w, 's> {
    game: Res<'w, GameMeta>,
    menu_page: ResMut<'w, MenuPage>,
    menu_input: Query<'w, 's, &'static mut ActionState<MenuAction>>,
    player_select_state: ResMut<'w, PlayerSelectState>,
    keyboard_input: Res<'w, Input<KeyCode>>,
    localization: Res<'w, Localization>,
}

impl<'w, 's> WidgetSystem for PlayerSelectMenu<'w, 's> {
    type Args = ();
    fn system(
        world: &mut World,
        state: &mut SystemState<Self>,
        ui: &mut egui::Ui,
        id: WidgetId,
        _: (),
    ) {
        let mut params: PlayerSelectMenu = state.get_mut(world);
        let is_online = false;

        handle_match_setup_messages(&mut params);

        // Whether or not the continue button should be enabled
        let mut ready_players = 0;
        let mut unconfirmed_players = 0;

        for slot in &params.player_select_state.slots {
            if slot.confirmed {
                ready_players += 1;
            } else if slot.active {
                unconfirmed_players += 1;
            }
        }
        let may_continue = ready_players >= 1 && unconfirmed_players == 0;

        // if let Some(client_info) = params.client_info {
        //     if may_continue {
        //         let player_to_select_map = *params
        //             .global_rng
        //             .sample(
        //                 &(0usize..client_info.player_count)
        //                     .into_iter()
        //                     .collect::<Vec<_>>(),
        //             )
        //             .unwrap();
        //         info!(%player_to_select_map, %client_info.player_idx);
        //         let is_waiting = player_to_select_map != client_info.player_idx;

        //         *params.menu_page = MenuPage::MapSelect { is_waiting };
        //     }
        // }

        ui.vertical_centered(|ui| {
            let params: PlayerSelectMenu = state.get_mut(world);

            let bigger_text_style = &params.game.ui_theme.font_styles.bigger;
            let heading_text_style = &params.game.ui_theme.font_styles.heading;
            let normal_button_style = &params.game.ui_theme.button_styles.normal;

            ui.add_space(heading_text_style.size / 4.0);
            if is_online {
                ui.themed_label(heading_text_style, &params.localization.get("online-game"));
            } else {
                ui.themed_label(heading_text_style, &params.localization.get("local-game"));
            }
            ui.themed_label(
                bigger_text_style,
                &params.localization.get("player-select-title"),
            );
            ui.add_space(normal_button_style.font.size);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                let mut params: PlayerSelectMenu = state.get_mut(world);

                let normal_button_style = &params.game.ui_theme.button_styles.normal;

                ui.add_space(normal_button_style.font.size * 2.0);

                ui.horizontal(|ui| {
                    // Calculate button size and spacing
                    let width = ui.available_width();
                    let button_width = width / 3.0;
                    let button_min_size = egui::vec2(button_width, 0.0);
                    let button_spacing = (width - 2.0 * button_width) / 3.0;

                    ui.add_space(button_spacing);

                    // Back button
                    let back_button = BorderedButton::themed(
                        &params.game.ui_theme.button_styles.normal,
                        &params.localization.get("back"),
                    )
                    .min_size(button_min_size)
                    .show(ui)
                    .focus_by_default(ui);

                    // Go to menu when back button is clicked
                    if back_button.clicked()
                        || (params.menu_input.single().just_pressed(MenuAction::Back)
                            && !params.player_select_state.slots[0].active)
                        || params.keyboard_input.just_pressed(KeyCode::Escape)
                    {
                        *params.menu_page = MenuPage::Home;
                        // if let Some(client) = params.client {
                        //     client.close();
                        // }
                        ui.ctx().clear_focus();
                    }

                    ui.add_space(button_spacing);

                    // Continue button
                    let continue_button = ui
                        .scope(|ui| {
                            ui.set_enabled(may_continue);

                            BorderedButton::themed(
                                &params.game.ui_theme.button_styles.normal,
                                &params.localization.get("continue"),
                            )
                            .min_size(button_min_size)
                            .show(ui)
                        })
                        .inner;

                    if continue_button.clicked()
                        || ((params.menu_input.single().just_pressed(MenuAction::Start)
                            || params.keyboard_input.just_pressed(KeyCode::Return))
                            && may_continue)
                    {
                        *params.menu_page = MenuPage::MapSelect { is_waiting: false };
                    }
                });

                ui.add_space(normal_button_style.font.size);

                ui.vertical_centered(|ui| {
                    let params: PlayerSelectMenu = state.get_mut(world);

                    let normal_button_style = &params.game.ui_theme.button_styles.normal;
                    ui.set_width(ui.available_width() - normal_button_style.font.size * 2.0);

                    ui.columns(MAX_PLAYERS, |columns| {
                        for (i, ui) in columns.iter_mut().enumerate() {
                            widget::<PlayerSelectPanel>(
                                world,
                                ui,
                                id.with(&format!("player_panel{i}")),
                                i,
                            );
                        }
                    });
                });
            });
        });
    }
}

fn handle_match_setup_messages(_params: &mut PlayerSelectMenu) {
    // if let Some(client) = &mut params.client {
    //     while let Some(message) = client.recv_reliable() {
    //         match message.kind {
    //             crate::networking::proto::ReliableGameMessageKind::MatchSetup(setup) => match setup
    //             {
    //                 MatchSetupMessage::SelectPlayer(player_handle) => {
    //                     params.player_inputs.players[message.from_player_idx].selected_player =
    //                         player_handle
    //                 }
    //                 MatchSetupMessage::ConfirmSelection(confirmed) => {
    //                     params.player_select_state.player_slots[message.from_player_idx]
    //                         .confirmed = confirmed;
    //                 }
    //                 MatchSetupMessage::SelectMap(_) => {
    //                     warn!("Unexpected map select message: player selection not yet confirmed");
    //                 }
    //             },
    //         }
    //     }
    // }
}

#[derive(SystemParam)]
struct PlayerSelectPanel<'w, 's> {
    game: Res<'w, GameMeta>,
    core: Res<'w, CoreMetaArc>,
    player_select_state: ResMut<'w, PlayerSelectState>,
    players: Query<
        'w,
        's,
        (
            &'static PlayerInputCollector,
            &'static ActionState<PlayerAction>,
        ),
    >,
    player_meta_assets: Res<'w, Assets<PlayerMeta>>,
    atlas_meta_assets: Res<'w, Assets<TextureAtlas>>,
    player_atlas_egui_textures: Res<'w, PlayerAtlasEguiTextures>,
    localization: Res<'w, Localization>,
}

impl<'w, 's> WidgetSystem for PlayerSelectPanel<'w, 's> {
    type Args = usize;
    fn system(
        world: &mut World,
        state: &mut SystemState<Self>,
        ui: &mut egui::Ui,
        _id: WidgetId,
        idx: usize,
    ) {
        let mut params: PlayerSelectPanel = state.get_mut(world);
        // let dummy_actions = default();

        let player_actions = params
            .players
            .iter()
            .find(|(player_idx, _)| player_idx.0 == idx)
            .unwrap()
            .1;
        // let player_actions = if let Some(match_info) = &params.client_match_info {
        //     if idx == match_info.player_idx {
        //         params
        //             .players
        //             .iter()
        //             .find(|(player_idx, _)| player_idx.0 == 0)
        //             .unwrap()
        //             .1
        //     } else {
        //         &dummy_actions
        //     }
        // } else {
        //     params
        //         .players
        //         .iter()
        //         .find(|(player_idx, _)| player_idx.0 == idx)
        //         .unwrap()
        //         .1
        // };

        let slot = &mut params.player_select_state.slots[idx];
        // if !player_input.active && params.client.is_some() {
        //     return;
        // }

        let player_handle = &mut slot.selected_player;

        // If the handle is empty
        if player_handle.path == default() {
            // Select the first player
            *player_handle = params.core.players[0].clone();
        }

        if player_actions.just_pressed(PlayerAction::Jump) {
            // if params.client.is_none() {
            if slot.active {
                slot.confirmed = true;
            } else {
                slot.active = true;
            }
            // } else {
            //     slot.confirmed = true;
            // }

            // if let Some(client) = params.client {
            //     client.send_reliable(
            //         MatchSetupMessage::ConfirmSelection(slot.confirmed),
            //         TargetClient::All,
            //     );
            // }
        } else if player_actions.just_pressed(PlayerAction::Grab) {
            // if params.client.is_none() {
            if slot.confirmed {
                slot.confirmed = false;
            } else {
                slot.active = false;
            }
            // } else {
            //     slot.confirmed = false;
            // }
            // if let Some(client) = params.client {
            //     client.send_reliable(
            //         MatchSetupMessage::ConfirmSelection(slot.confirmed),
            //         TargetClient::All,
            //     );
            // }
        } else if player_actions.just_pressed(PlayerAction::Move) && !slot.confirmed {
            let direction = player_actions
                .clamped_axis_pair(PlayerAction::Move)
                .unwrap();

            let current_player_handle_idx = params
                .core
                .players
                .iter()
                .enumerate()
                .find(|(_, handle)| handle.path == player_handle.path)
                .map(|(i, _)| i)
                .unwrap_or(0);

            if direction.x() > 0.0 {
                *player_handle = params
                    .core
                    .players
                    .get(current_player_handle_idx + 1)
                    .cloned()
                    .unwrap_or_else(|| params.core.players[0].clone());
            } else if direction.x() <= 0.0 {
                if current_player_handle_idx > 0 {
                    *player_handle = params
                        .core
                        .players
                        .get(current_player_handle_idx - 1)
                        .cloned()
                        .unwrap();
                } else {
                    *player_handle = params.core.players.iter().last().unwrap().clone();
                }
            }

            // if let Some(client) = params.client {
            //     client.send_reliable(
            //         MatchSetupMessage::SelectPlayer(player_handle.clone_weak()),
            //         TargetClient::All,
            //     );
            // }
        }

        BorderedFrame::new(&params.game.ui_theme.panel.border)
            .padding(params.game.ui_theme.panel.padding.into())
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                let normal_font = &params.game.ui_theme.font_styles.normal;
                let heading_font = &params.game.ui_theme.font_styles.heading;

                // // Marker for current player in online matches
                // if let Some(match_info) = params.client_match_info {
                //     if match_info.player_idx == idx {
                //         ui.vertical_centered(|ui| {
                //             ui.themed_label(normal_font, &params.localization.get("you-marker"));
                //         });
                //     } else {
                //         ui.add_space(normal_font.size);
                //     }
                // } else {
                    ui.add_space(normal_font.size);
                // }

                if slot.active {
                    ui.vertical_centered(|ui| {
                        let Some(player_meta) = params.player_meta_assets.get(&player_handle.get_bevy_handle()) else { return; };
                        let Some(atlas_meta) = params.atlas_meta_assets.get(&player_meta.atlas.get_bevy_handle_untyped().typed()) else { return; };

                        ui.themed_label(normal_font, &params.localization.get("pick-a-fish"));

                        ui.vertical_centered(|ui| {
                            ui.set_height(heading_font.size * 1.5);

                            if slot.confirmed {
                                ui.themed_label(
                                    &heading_font.colored(params.game.ui_theme.colors.positive),
                                    &params.localization.get("player-select-ready"),
                                );
                            }
                        });

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                            let name_with_arrows = format!("<  {}  >", player_meta.name);
                            ui.themed_label(
                                normal_font,
                                if slot.confirmed {
                                    &player_meta.name
                                } else {
                                    &name_with_arrows
                                },
                            );

                            player_image(ui, player_handle, player_meta, atlas_meta, &params.player_atlas_egui_textures);
                        });
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.themed_label(
                            normal_font,
                            &params.localization.get("press-jump-to-join"),
                        );
                    });
                }
            });
    }
}

#[derive(Resource)]
pub struct PlayerAtlasEguiTextures(pub HashMap<bones::AssetPath, egui::TextureId>);

fn player_image(
    ui: &mut egui::Ui,
    player_handle: &bones::Handle<PlayerMeta>,
    player_meta: &PlayerMeta,
    atlas: &TextureAtlas,
    egui_textures: &PlayerAtlasEguiTextures,
) {
    let time = ui.ctx().input().time as f32;
    let anim_clip = player_meta
        .animations
        .get(&key!("idle"))
        .expect("Missing `idle` animation");
    let fps = anim_clip.fps;
    let frame_in_time_idx = (time * fps).round() as usize;
    let frame_in_clip_idx = frame_in_time_idx % (anim_clip.end - anim_clip.start);
    let frame_in_sheet_idx = anim_clip.start + frame_in_clip_idx;
    let sprite_rect = &atlas.textures[frame_in_sheet_idx];

    let sprite_aspect = sprite_rect.height() / sprite_rect.width();
    let width = ui.available_width();
    let height = sprite_aspect * width;
    let available_height = ui.available_height();
    let y_offset = -(available_height - height) / 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    let uv_min = sprite_rect.min / atlas.size;
    let uv_max = sprite_rect.max / atlas.size;
    let uv = egui::Rect {
        min: egui::pos2(uv_min.x, uv_min.y),
        max: egui::pos2(uv_max.x, uv_max.y),
    };

    let mut mesh = egui::Mesh {
        texture_id: *egui_textures.0.get(&player_handle.path).unwrap(),
        ..default()
    };

    mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);

    mesh.translate(egui::vec2(0.0, y_offset));

    ui.painter().add(mesh);
}
