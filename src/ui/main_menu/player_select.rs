use crate::loading::PlayerInputCollector;
#[cfg(not(target_arch = "wasm32"))]
use crate::networking::{NetworkMatchSocket, SocketTarget};

use bones_lib::prelude::{key, Key, KeyError};
use rand::Rng;

use super::*;

const GAMEPAD_ACTION_IDX: usize = 0;
const KEYPAD_ACTION_IDX: usize = 1;

#[derive(Resource, Default)]
pub struct PlayerSelectState {
    pub slots: [PlayerSlot; MAX_PLAYERS],
}

#[derive(Default)]
pub struct PlayerSlot {
    pub active: bool,
    pub confirmed: bool,
    pub selected_player: bones::Handle<PlayerMeta>,
    pub is_ai: bool,
}

/// Network message that may be sent during player selection.
#[derive(Serialize, Deserialize)]
pub enum PlayerSelectMessage {
    SelectPlayer(bones::Handle<PlayerMeta>),
    ConfirmSelection(bool),
}

#[derive(SystemParam)]
pub struct PlayerSelectMenu<'w, 's> {
    game: Res<'w, GameMeta>,
    menu_page: ResMut<'w, MenuPage>,
    localization: Res<'w, Localization>,
    keyboard_input: Res<'w, Input<KeyCode>>,
    player_select_state: ResMut<'w, PlayerSelectState>,
    menu_input: Query<'w, 's, &'static mut ActionState<MenuAction>>,
    #[cfg(not(target_arch = "wasm32"))]
    network_socket: Option<Res<'w, NetworkMatchSocket>>,
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
        #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
        let mut params: PlayerSelectMenu = state.get_mut(world);
        let is_online = false;

        #[cfg(not(target_arch = "wasm32"))]
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

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(socket) = &params.network_socket {
            if may_continue {
                // The first player picks the map
                let is_waiting = socket.player_idx() != 0;

                *params.menu_page = MenuPage::MapSelect { is_waiting };
            }
        }

        ui.vertical_centered(|ui| {
            let params: PlayerSelectMenu = state.get_mut(world);

            let bigger_text_style = &params.game.ui_theme.font_styles.bigger;
            let heading_text_style = &params.game.ui_theme.font_styles.heading;
            let normal_button_style = &params.game.ui_theme.button_styles.normal;

            ui.add_space(heading_text_style.size / 4.0);

            // Title
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
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(socket) = params.network_socket {
                            socket.close();
                        }
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

#[cfg(not(target_arch = "wasm32"))]
fn handle_match_setup_messages(params: &mut PlayerSelectMenu) {
    if let Some(socket) = &params.network_socket {
        let datas: Vec<(usize, Vec<u8>)> = socket.recv_reliable();

        for (player, data) in datas {
            match postcard::from_bytes::<PlayerSelectMessage>(&data) {
                Ok(message) => match message {
                    PlayerSelectMessage::SelectPlayer(player_handle) => {
                        params.player_select_state.slots[player].selected_player = player_handle
                    }
                    PlayerSelectMessage::ConfirmSelection(confirmed) => {
                        params.player_select_state.slots[player].confirmed = confirmed;
                    }
                },
                Err(e) => warn!("Ignoring network message that was not understood: {e}"),
            }
        }
    }
}

#[derive(Debug)]
struct PlayerActionMap<'a>(HashMap<PlayerAction, Vec<Option<&'a UserInput>>>);

impl PlayerActionMap<'_> {
    fn get_text(&self, action: PlayerAction) -> String {
        self.0
            .get(&action)
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|action| *action)
            .map(|action| action.to_string())
            .fold("".to_string(), |acc, curr| {
                if acc.is_empty() {
                    curr
                } else {
                    format!("{acc} / {curr}")
                }
            })
    }
}

fn get_player_actions(idx: usize, map: &InputMap<PlayerAction>) -> PlayerActionMap {
    let map_idx = if idx > 1 {
        GAMEPAD_ACTION_IDX
    } else {
        KEYPAD_ACTION_IDX
    };

    let mut jump_actions = vec![get_user_action(map_idx, PlayerAction::Jump, map)];
    let mut grab_actions = vec![get_user_action(map_idx, PlayerAction::Grab, map)];

    if idx <= 1 {
        jump_actions.push(get_user_action(GAMEPAD_ACTION_IDX, PlayerAction::Jump, map));
        grab_actions.push(get_user_action(GAMEPAD_ACTION_IDX, PlayerAction::Grab, map));
    }

    PlayerActionMap(HashMap::from_iter(vec![
        (PlayerAction::Jump, jump_actions),
        (PlayerAction::Grab, grab_actions),
    ]))
}

fn get_user_action(
    idx: usize,
    action: PlayerAction,
    map: &InputMap<PlayerAction>,
) -> Option<&'_ UserInput> {
    let action = map.get(action).get_at(idx);
    if let Some(action) = action {
        Some(action)
    } else {
        None
    }
}

#[derive(SystemParam)]
struct PlayerSelectPanel<'w, 's> {
    game: Res<'w, GameMeta>,
    core: Res<'w, CoreMetaArc>,
    localization: Res<'w, Localization>,
    player_meta_assets: Res<'w, Assets<PlayerMeta>>,
    player_select_state: ResMut<'w, PlayerSelectState>,
    atlas_meta_assets: Res<'w, Assets<TextureAtlas>>,
    player_atlas_egui_textures: Res<'w, PlayerAtlasEguiTextures>,
    players: Query<
        'w,
        's,
        (
            &'static PlayerInputCollector,
            &'static ActionState<PlayerAction>,
            &'static InputMap<PlayerAction>,
        ),
    >,
    #[cfg(not(target_arch = "wasm32"))]
    network_socket: Option<Res<'w, NetworkMatchSocket>>,
}

impl<'w, 's> WidgetSystem for PlayerSelectPanel<'w, 's> {
    type Args = usize; // Player Idx
    fn system(
        world: &mut World,
        state: &mut SystemState<Self>,
        ui: &mut egui::Ui,
        _id: WidgetId,
        args: usize,
    ) {
        let mut params: PlayerSelectPanel = state.get_mut(world);

        #[cfg(target_arch = "wasm32")]
        let is_network = false;
        #[cfg(not(target_arch = "wasm32"))]
        let is_network = params.network_socket.is_some();

        let player_id = args;

        let player_map = params
            .players
            .iter()
            .find(|(player_idx, _, _)| player_idx.0 == player_id)
            .unwrap()
            .2;

        #[cfg(not(target_arch = "wasm32"))]
        let dummy_actions = default();
        let (player_actions, player_action_map) = if is_network {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(socket) = &params.network_socket {
                let actions = if player_id == socket.player_idx() {
                    params
                        .players
                        .iter()
                        .find(|(player_idx, _, _)| player_idx.0 == 0)
                        .unwrap()
                        .1
                } else {
                    &dummy_actions
                };
                let map = None;
                (actions, map)
            } else {
                unreachable!();
            }

            #[cfg(target_arch = "wasm32")]
            unreachable!()
        } else {
            let actions = params
                .players
                .iter()
                .find(|(player_idx, _, _)| player_idx.0 == player_id)
                .unwrap()
                .1;
            let map = Some(get_player_actions(player_id, player_map));
            (actions, map)
        };

        let slot = &mut params.player_select_state.slots[player_id];
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(socket) = &params.network_socket {
            // Don't show panels for non-connected players.
            if player_id + 1 > socket.player_count() {
                return;
            } else {
                slot.active = true;
            }
        }

        let player_handle = &mut slot.selected_player;

        // If the handle is empty
        if player_handle.path == default() {
            // Select the first player
            *player_handle = params.core.players[0].clone();
        }

        if player_actions.just_pressed(PlayerAction::Jump) {
            if !is_network {
                if slot.active {
                    slot.confirmed = true;
                } else {
                    slot.active = true;
                }
            } else {
                slot.confirmed = true;
            }

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(socket) = &params.network_socket {
                socket.send_reliable(
                    SocketTarget::All,
                    &postcard::to_allocvec(&PlayerSelectMessage::ConfirmSelection(slot.confirmed))
                        .unwrap(),
                );
            }
        } else if player_actions.just_pressed(PlayerAction::Grab) {
            if !is_network {
                if slot.confirmed {
                    slot.confirmed = false;
                } else {
                    slot.active = false;
                }
            } else {
                slot.confirmed = false;
            }

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(socket) = &params.network_socket {
                socket.send_reliable(
                    SocketTarget::All,
                    &postcard::to_allocvec(&PlayerSelectMessage::ConfirmSelection(slot.confirmed))
                        .unwrap(),
                );
            }
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

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(socket) = &params.network_socket {
                socket.send_reliable(
                    SocketTarget::All,
                    &postcard::to_allocvec(&PlayerSelectMessage::SelectPlayer(
                        player_handle.clone(),
                    ))
                    .unwrap(),
                );
            }
        }

        BorderedFrame::new(&params.game.ui_theme.panel.border)
            .padding(params.game.ui_theme.panel.padding.into())
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                let normal_font = &params.game.ui_theme.font_styles.normal;
                let heading_font = &params.game.ui_theme.font_styles.heading;

                // Marker for current player in online matches
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(socket) = &params.network_socket {
                    if socket.player_idx() == player_id {
                        ui.vertical_centered(|ui| {
                            ui.themed_label(normal_font, &params.localization.get("you-marker"));
                        });
                    } else {
                        ui.add_space(normal_font.size);
                    }
                } else {
                    ui.add_space(normal_font.size);
                }
                #[cfg(target_arch = "wasm32")]
                ui.add_space(normal_font.size);

                if slot.active {
                    ui.vertical_centered(|ui| {
                        let Some(player_meta) = params
                            .player_meta_assets
                            .get(&player_handle.get_bevy_handle()) else { return; };

                        ui.themed_label(normal_font, &params.localization.get("pick-a-fish"));

                        if !slot.confirmed {
                            if let Some(player_action_map) = &player_action_map {
                                ui.themed_label(
                                    normal_font,
                                    &params.localization.get(&format!(
                                        "press-button-to-lock-in?button={}",
                                        player_action_map.get_text(PlayerAction::Jump)
                                    )),
                                );

                                ui.themed_label(
                                    normal_font,
                                    &params.localization.get(&format!(
                                        "press-button-to-remove?button={}",
                                        player_action_map.get_text(PlayerAction::Grab)
                                    )),
                                );
                            } else {
                                ui.themed_label(normal_font, &params.localization.get("waiting"));
                            }
                        }

                        ui.vertical_centered(|ui| {
                            ui.set_height(heading_font.size * 1.5);

                            if slot.confirmed && !slot.is_ai {
                                ui.themed_label(
                                    &heading_font.colored(params.game.ui_theme.colors.positive),
                                    &params.localization.get("player-select-ready"),
                                );

                                if let Some(player_action_map) = &player_action_map {
                                    ui.themed_label(
                                        normal_font,
                                        &params.localization.get(&format!(
                                            "player-select-unready?button={}",
                                            player_action_map.get_text(PlayerAction::Grab)
                                        )),
                                    );
                                }
                            }
                            if slot.is_ai {
                                ui.themed_label(
                                    &heading_font.colored(params.game.ui_theme.colors.positive),
                                    &params.localization.get("ai-player"),
                                );
                                if BorderedButton::themed(
                                    &params.game.ui_theme.button_styles.normal,
                                    &params.localization.get("remove-ai-player"),
                                )
                                .show(ui)
                                .clicked()
                                {
                                    slot.confirmed = false;
                                    slot.active = false;
                                    slot.is_ai = false;
                                }
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

                            player_image(
                                ui,
                                player_meta,
                                &params.atlas_meta_assets,
                                &params.player_atlas_egui_textures,
                            );
                        });
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        if let Some(player_action_map) = &player_action_map {
                            ui.themed_label(
                                normal_font,
                                &params.localization.get(&format!(
                                    "press-button-to-join?button={}",
                                    player_action_map.get_text(PlayerAction::Jump)
                                )),
                            );
                        }

                        if player_id != 0 && !is_network {
                            ui.add_space(params.game.ui_theme.font_styles.bigger.size);
                            if BorderedButton::themed(
                                &params.game.ui_theme.button_styles.normal,
                                &params.localization.get("add-ai-player"),
                            )
                            .show(ui)
                            .clicked()
                            {
                                slot.is_ai = true;
                                slot.confirmed = true;
                                slot.active = true;
                                let mut rng = rand::thread_rng();
                                *player_handle = params.core.players
                                    [rng.gen_range(0..params.core.players.len())]
                                .clone();
                            }
                        }
                    });
                }
            });
    }
}

#[derive(Resource)]
pub struct PlayerAtlasEguiTextures(pub HashMap<bones::AssetPath, egui::TextureId>);

fn player_image(
    ui: &mut egui::Ui,
    player_meta: &PlayerMeta,
    atlas_assets: &Assets<TextureAtlas>,
    egui_textures: &PlayerAtlasEguiTextures,
) {
    let time = ui.ctx().input(|i| i.time as f32);
    let width = ui.available_width();
    let available_height = ui.available_width();

    let body_rect;
    let body_scale;
    let body_offset;
    let y_offset;
    // Render the body sprite
    {
        let atlas_handle = &player_meta.layers.body.atlas;
        let atlas = atlas_assets
            .get(&atlas_handle.get_bevy_handle_untyped().typed())
            .unwrap();
        let atlas_path = &atlas_handle.path;
        let anim_clip = player_meta
            .layers
            .body
            .animations
            .frames
            .get(&key!("idle"))
            .unwrap();
        let fps = anim_clip.fps;
        let frame_in_time_idx = (time * fps).round() as usize;
        let frame_in_clip_idx = frame_in_time_idx % anim_clip.frames.len();
        let frame_in_sheet_idx = anim_clip.frames[frame_in_clip_idx];
        let sprite_rect = &atlas.textures[frame_in_sheet_idx];
        body_offset =
            player_meta.layers.body.animations.body_offsets[&key!("idle")][frame_in_clip_idx];

        let sprite_aspect = sprite_rect.height() / sprite_rect.width();
        let height = sprite_aspect * width;
        y_offset = -(available_height - height) / 2.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

        let uv_min = sprite_rect.min / atlas.size;
        let uv_max = sprite_rect.max / atlas.size;
        let uv = egui::Rect {
            min: egui::pos2(uv_min.x, uv_min.y),
            max: egui::pos2(uv_max.x, uv_max.y),
        };

        let mut mesh = egui::Mesh {
            texture_id: *egui_textures.0.get(atlas_path).unwrap(),
            ..default()
        };

        mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
        mesh.translate(egui::vec2(0.0, y_offset));
        ui.painter().add(mesh);

        body_rect = rect;
        body_scale = width / sprite_rect.size().x;
    }

    // Render the fin animation
    {
        let atlas_handle = &player_meta.layers.fin.atlas;
        let atlas = atlas_assets
            .get(&atlas_handle.get_bevy_handle_untyped().typed())
            .unwrap();
        let atlas_path = &atlas_handle.path;
        let anim_clip = player_meta
            .layers
            .fin
            .animations
            .get(&key!("idle"))
            .unwrap();
        let fps = anim_clip.fps;
        let frame_in_time_idx = (time * fps).round() as usize;
        let frame_in_clip_idx = frame_in_time_idx % anim_clip.frames.len();
        let frame_in_sheet_idx = anim_clip.frames[frame_in_clip_idx];
        let sprite_rect = &atlas.textures[frame_in_sheet_idx];

        let uv_min = sprite_rect.min / atlas.size;
        let uv_max = sprite_rect.max / atlas.size;
        let uv = egui::Rect {
            min: egui::pos2(uv_min.x, uv_min.y),
            max: egui::pos2(uv_max.x, uv_max.y),
        };

        let mut mesh = egui::Mesh {
            texture_id: *egui_textures.0.get(atlas_path).unwrap(),
            ..default()
        };

        let sprite_size = sprite_rect.size() * body_scale;
        let offset = (player_meta.layers.fin.offset + body_offset) * body_scale;
        let rect = egui::Rect::from_center_size(
            body_rect.center() + egui::vec2(offset.x, -offset.y + y_offset),
            egui::vec2(sprite_size.x, sprite_size.y),
        );

        mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
        ui.painter().add(mesh);
    }

    // Render face animation
    {
        let atlas_handle = &player_meta.layers.face.atlas;
        let atlas = atlas_assets
            .get(&atlas_handle.get_bevy_handle_untyped().typed())
            .unwrap();
        let atlas_path = &atlas_handle.path;
        let anim_clip = player_meta
            .layers
            .face
            .animations
            .get(&key!("idle"))
            .unwrap();
        let fps = anim_clip.fps;
        let frame_in_time_idx = (time * fps).round() as usize;
        let frame_in_clip_idx = frame_in_time_idx % anim_clip.frames.len();
        let frame_in_sheet_idx = anim_clip.frames[frame_in_clip_idx];
        let sprite_rect = &atlas.textures[frame_in_sheet_idx];

        let uv_min = sprite_rect.min / atlas.size;
        let uv_max = sprite_rect.max / atlas.size;
        let uv = egui::Rect {
            min: egui::pos2(uv_min.x, uv_min.y),
            max: egui::pos2(uv_max.x, uv_max.y),
        };

        let mut mesh = egui::Mesh {
            texture_id: *egui_textures.0.get(atlas_path).unwrap(),
            ..default()
        };

        let sprite_size = sprite_rect.size() * body_scale;
        let offset = (player_meta.layers.face.offset + body_offset) * body_scale;
        let rect = egui::Rect::from_center_size(
            body_rect.center() + egui::vec2(offset.x, -offset.y + y_offset),
            egui::vec2(sprite_size.x, sprite_size.y),
        );

        mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
        ui.painter().add(mesh);
    }
}
