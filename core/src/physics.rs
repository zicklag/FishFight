//! Physics and collision detection.

use crate::prelude::*;

pub use collisions::{
    Actor, Collider, ColliderShape, CollisionWorld, RapierContext, RapierUserData,
    TileCollisionKind,
};

pub mod collisions;

#[derive(Debug, Clone, Copy)]
enum PhysicsStage {
    Update,
}

impl StageLabel for PhysicsStage {
    fn name(&self) -> String {
        format!("{self:?}")
    }

    fn id(&self) -> Ulid {
        match self {
            PhysicsStage::Update => Ulid(2026032502318358068964002697585621138),
        }
    }
}

pub fn install(session: &mut CoreSession) {
    session
        .stages
        // TODO: Think again about exactly how to organize the physics sync systems. At the time of
        // writing, we have to do an extra `collision_world.step()` when debug rendering to make
        // sure everything is synced. It'd be good to avoid this and maybe take inspiration from
        // bevy_rapier.
        .insert_stage_after(
            CoreStage::PostUpdate,
            SimpleSystemStage::new(PhysicsStage::Update),
        )
        .add_system_to_stage(PhysicsStage::Update, hydrate_physics_bodies)
        .add_system_to_stage(PhysicsStage::Update, update_kinematic_bodies);
}

/// A kinematic physics body
///
/// Used primarily for players and things that need to walk around, detect what kind of platform
/// they are standing on, etc.
///
/// For now, all kinematic bodies have axis-aligned, rectangular colliders. This may or may not
/// change in the future.
#[derive(Default, Debug, Clone, Copy, TypeUlid)]
#[ulid = "01GNF5NRJR7CXZ9NKEBQEGN8R1"]
#[repr(C)]
pub struct KinematicBody {
    pub velocity: Vec2,
    pub shape: ColliderShape,
    /// Angular velocity in degrees per second
    pub angular_velocity: f32,
    pub gravity: f32,
    pub bounciness: f32,
    /// Sets a 1 frame override for the body friction. It will be re-set to `None` every frame so if
    /// you wish to apply a continuous friction change, you must re-set it every frame.
    ///
    /// This is useful for things like slippery blocks or other things that want to modify a body's
    /// friction while it is on the block.
    pub frame_friction_override: Option<f32>,
    pub is_on_ground: bool,
    pub was_on_ground: bool,
    /// Will be `true` if the body is currently on top of a platform/jumpthrough tile
    pub is_on_platform: bool,
    /// If this is `true` the body will be affected by gravity
    pub has_mass: bool,
    pub has_friction: bool,
    pub can_rotate: bool,
    pub is_deactivated: bool,
    /// Whether or not the body should fall through jump_through platforms
    pub fall_through: bool,
    /// Indicates that we should reset the collider like it was just added to the world.
    ///
    /// This is important to make sure that it falls through JumpThrough platforms if it happens to
    /// spawn inside of one.
    pub is_spawning: bool,
}

impl KinematicBody {
    /// Get the body's axis-aligned-bounding-box ( AABB ).
    ///
    /// An AABB is the smallest non-rotated rectangle that completely contains the the collision
    /// shape.
    ///
    /// By passing in the body's global transform you will get the world-space bounding box.
    pub fn bounding_box(&self, transform: Transform) -> Rect {
        let aabb = self.shape.compute_aabb(transform);

        Rect {
            min: vec2(aabb.mins.x, aabb.mins.y),
            max: vec2(aabb.maxs.x, aabb.maxs.y),
        }
    }
}

/// Hydrate newly added [`KinematicBody`]s.
fn hydrate_physics_bodies(
    entities: Res<Entities>,
    bodies: Comp<KinematicBody>,
    mut collision_world: CollisionWorld,
) {
    let mut needs_hydrate = collision_world.colliders.bitset().clone();
    needs_hydrate.bit_not();
    needs_hydrate.bit_and(bodies.bitset());

    for entity in entities.iter_with_bitset(&needs_hydrate) {
        let body = bodies.get(entity).unwrap();

        collision_world.colliders.insert(
            entity,
            Collider {
                shape: body.shape,
                ..default()
            },
        );
        collision_world.actors.insert(entity, Actor);
        collision_world.handle_teleport(entity);
    }
}

/// Update physics for kinematic bodies.
fn update_kinematic_bodies(
    game: Res<CoreMetaArc>,
    entities: Res<Entities>,
    mut bodies: CompMut<KinematicBody>,
    mut collision_world: CollisionWorld,
    mut transforms: CompMut<Transform>,
    time: Res<Time>,
) {
    puffin::profile_function!();

    // TODO: By default we assume a frame rate of of crate::FPS ( 60 frames-per-second ), but we may
    // actually run at a different rate during, for example, network games. This gets the ratio
    // between the default frame rate and the actual frame rate and is used to adjust velocities.
    // This is a temporary fix. In the future, we should adjust all velocities to be in terms of
    // pixels-per-second, instead of pixels-per-1/60 of a second like they are today. This temporary
    // fix allows us to avoid chainging the velocities throughout the game for now.
    let time_factor = time.delta().as_secs_f32() / (1.0 / crate::FPS);

    collision_world.update(&transforms);
    for (entity, body) in entities.iter_with(&mut bodies) {
        if body.is_deactivated {
            collision_world.colliders.get_mut(entity).unwrap().disabled = true;
            continue;
        } else {
            collision_world.colliders.get_mut(entity).unwrap().disabled = false;
        }

        if body.has_mass {
            puffin::profile_scope!("Shove objects out of walls");

            // Shove objects out of walls
            loop {
                let mut transform = transforms.get(entity).copied().unwrap();

                if collision_world.tile_collision(transform, body.shape) != TileCollisionKind::Solid
                {
                    break;
                }

                let rect = body.bounding_box(transform);
                // We add a small border, because rapier will consider the collision box colliding
                // if it is perfectly lined up along the edge of a tile, and `solid_at` won't.
                let border = 0.1;

                let collisions = (
                    collision_world.solid_at(vec2(rect.min.x - border, rect.max.y + border)), // Top left
                    collision_world.solid_at(vec2(rect.max.x + border, rect.max.y + border)), // Top right
                    collision_world.solid_at(vec2(rect.max.x + border, rect.min.y - border)), // Bottom right
                    collision_world.solid_at(vec2(rect.min.x - border, rect.min.y - border)), // Bottom left
                );
                match collisions {
                    // If we have no solid collisions at any corner.
                    (false, false, false, false) => {
                        // For some reason the `tile_collision` test did detect a collision, but
                        // `solid_at` did not detect a collision at any of the corners of the aabb.
                        warn!(
                            "Collision test error resulting in physics \
                            body stuck in wall at {rect:?}",
                        );
                        break;
                    }
                    // Check for collisions on each side of the rectangle
                    (false, false, _, _) => transform.translation.y += 1.0,
                    (_, false, false, _) => transform.translation.x += 1.0,
                    (_, _, false, false) => transform.translation.y -= 1.0,
                    (false, _, _, false) => transform.translation.x -= 1.0,
                    // If none of the sides of the rectangle are un-collided, then we don't know
                    // which direction to move to get out of the wall, and we just give up.
                    _ => {
                        break;
                    }
                }

                *transforms.get_mut(entity).unwrap() = transform;
            }
        }

        // Sync body attributes with collider
        {
            let collider = collision_world.colliders.get_mut(entity).unwrap();
            collider.shape = body.shape;
        }

        if body.is_spawning {
            collision_world.handle_teleport(entity);
            body.is_spawning = false;
        }

        if body.fall_through {
            collision_world.descent(entity);
        }

        {
            puffin::profile_scope!("move body");

            if collision_world.move_vertical(&mut transforms, entity, body.velocity.y * time_factor)
            {
                body.velocity.y *= -body.bounciness;
            }

            // NOTE: It's important that we move horizontally after we move vertically, or else the
            // horizontal movement will clear our `descent` and `seen_wood` flags and we may not go
            // through drop through platforms while moving horizontally.
            if collision_world.move_horizontal(
                &mut transforms,
                entity,
                body.velocity.x * time_factor,
            ) {
                body.velocity.x *= -body.bounciness;
            }
        }

        // Check ground collision
        {
            let mut transform = transforms.get(entity).copied().unwrap();

            // Don't get stuck floating in fall-through platforms
            if body.velocity == Vec2::ZERO
                && collision_world.tile_collision_filtered(transform, body.shape, |ent| {
                    collision_world
                        .tile_collision_kinds
                        .get(ent)
                        .map(|x| *x == TileCollisionKind::JumpThrough)
                        .unwrap_or(false)
                }) == TileCollisionKind::JumpThrough
            {
                body.fall_through = true;
            }

            // Move transform check down 1 slightly
            transform.translation.y -= 0.1;

            body.was_on_ground = body.is_on_ground;

            let collider = collision_world.get_collider(entity);

            let tile = collision_world.tile_collision_filtered(transform, body.shape, |ent| {
                if collider.seen_wood {
                    collision_world
                        .tile_collision_kinds
                        .get(ent)
                        .map(|x| *x != TileCollisionKind::JumpThrough)
                        .unwrap_or(false)
                } else {
                    true
                }
            });

            let on_jump_through_tile = tile == TileCollisionKind::JumpThrough;
            body.is_on_ground =
                tile != TileCollisionKind::Empty && !(on_jump_through_tile && body.fall_through);
            body.is_on_platform = body.is_on_ground && on_jump_through_tile;
        }

        if body.is_on_ground {
            if body.has_friction {
                body.velocity.x *= if let Some(friction) = body.frame_friction_override {
                    friction
                } else {
                    game.physics.friction_lerp
                };
                body.frame_friction_override = None;

                if body.velocity.x.abs() <= game.physics.stop_threshold {
                    body.velocity.x = 0.0;
                }

                body.velocity.y *= game.physics.friction_lerp;
            }

            if body.velocity.y <= game.physics.gravity {
                body.velocity.y = 0.0;
            }
        }

        if !body.is_on_ground && body.has_mass {
            body.velocity.y -= body.gravity * time_factor;

            if body.velocity.y < -game.physics.terminal_velocity {
                body.velocity.y = -game.physics.terminal_velocity;
            }
        }

        if body.can_rotate {
            apply_rotation(
                transforms.get_mut(entity).unwrap(),
                body.velocity,
                body.angular_velocity,
                body.is_on_ground,
                body.shape,
            );
        }
    }
}

/// Helper function to apply rotation to a kinematic body.
fn apply_rotation(
    transform: &mut Transform,
    velocity: Vec2,
    angular_velocity: f32,
    is_on_ground: bool,
    collider_shape: ColliderShape,
) {
    puffin::profile_function!();

    let mut angle = transform.rotation.to_euler(EulerRot::XYZ).2;

    if is_on_ground {
        if matches!(collider_shape, ColliderShape::Circle { .. }) {
            angle += velocity.x.abs() * angular_velocity;
        }
    } else {
        angle += (angular_velocity * crate::FPS).to_radians();
    }

    transform.rotation = Quat::from_rotation_z(angle);
}
