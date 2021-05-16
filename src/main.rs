use bevy::{
    prelude::*,
    render::camera::Camera,
    input::mouse::{MouseWheel, MouseMotion},
};
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::build()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(Msaa { samples: 8 })
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_startup_system(setup.system())
        .add_startup_system(cam_setup.system())
        .add_system(cam.system())
        .add_system_set(
            SystemSet::new()
                .with_system(
                    update_acceleration
                        .system()
                        .label(PhysicsSystem::UpdateAcceleration),
                )
                .with_system(
                    update_velocity
                        .system()
                        .label(PhysicsSystem::UpdateVelocity)
                        .after(PhysicsSystem::UpdateAcceleration),
                )
                .with_system(
                    movement
                        .system()
                        .label(PhysicsSystem::Movement)
                        .after(PhysicsSystem::UpdateVelocity),
                )
        )
        .run();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum PhysicsSystem {
    UpdateAcceleration,
    UpdateVelocity,
    Movement,
}

struct Mass(f32);
struct Velocity(Vec2);
struct Acceleration(Vec2);

#[derive(Bundle)]
struct BodyBundle {
    mass: Mass,
    transform: Transform,
    velocity: Velocity,
    acceleration: Acceleration,
}

impl BodyBundle {
    fn new(mass: f32, pos: Vec2, vel: Vec2) -> Self {
        Self {
            mass: Mass(mass),
            transform: Transform::from_translation(Vec3::new(pos[0], pos[1], 1.0)),
            velocity: Velocity(vel),
            acceleration: Acceleration(Vec2::new(0.0, 0.0)),
        }
    }
}

struct BodyTemplate {
    mass: f32,
    radius: f32,
    color: Color,
    pos: Vec2,
    vel: Vec2
}

impl BodyTemplate {
    fn new(mass: f32, density: f32, color: Color, pos: Vec2, vel: Vec2) -> Self {
        BodyTemplate {
            mass: mass,
            radius: mass / density,
            color: color,
            pos: pos,
            vel: vel,
        }
    }
}

struct GameCam;

fn setup(
    mut commands: Commands,
    // asset_server: Res<AssetServer>,
    // mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d()).insert(GameCam);
    // commands.spawn_bundle(UiCameraBundle::default());

    // commands
    //     .spawn_bundle(NodeBundle {
    //         style: Style {
    //             size: Size::new(Val::Percent(20.0), Val::Percent(100.0)),
    //             ..Default::default()
    //         },
    //         material: materials.add(Color::DARK_GRAY.into()),
    //         ..Default::default()
    //     });

    let bodies = vec![
        BodyTemplate::new(200.0, 10.0, Color::YELLOW, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0)),
        BodyTemplate::new(50.0, 5.0, Color::BLUE, Vec2::new(100.0, 0.0), Vec2::new(0.0, -1.0)),
        BodyTemplate::new(50.0, 5.0, Color::RED, Vec2::new(-100.0, 0.0), Vec2::new(0.0, 1.0)),
        // BodyTemplate::new(50.0, 5.0, Color::GREEN, Vec2::new(0.0, 350.0), Vec2::new(0.0, 0.0)),
        // BodyTemplate::new(40.0, 0.0, Color::WHITE, Vec2::new(-80.0, 80.0), Vec2::new(0.0, 0.0)),
    ];

    for body in bodies.iter() {
        commands.spawn_bundle(GeometryBuilder::build_as(
            &shapes::Circle {
                radius: body.radius,
                center: body.pos,
                ..shapes::Circle::default()
            },
            ShapeColors::outlined(body.color, body.color),
            DrawMode::Outlined {
                fill_options: FillOptions::default(),
                outline_options: StrokeOptions::default(),
            },
            Transform::default(),
        )).insert_bundle(BodyBundle::new(
            body.mass,
            body.pos,
            body.vel,
        ));
    }
}

fn cam_setup(
    mut camera_query: Query<(&mut Camera, &mut Transform)>
) {
    for (_cam, mut trans) in camera_query.iter_mut() {
        trans.scale = Vec3::new(10.0, 10.0, 1.0);
    }
}

const ZOOM_SENSITIVITY: f32 = 0.1;
const DT: f32 = 1.5;

fn update_acceleration(
    mut query: Query<(Entity, &Mass, &Velocity, &mut Acceleration, &Transform)>
) {
    let mut bodies: Vec<(&Mass, &Transform, Mut<Acceleration>)> = Vec::new();
    for (_ent, mass, _vel, mut acc, trans) in query.iter_mut() {
        for (mass2, trans2, acc2) in bodies.iter_mut() {
            let diff = trans.translation - trans2.translation;
            // if mass.0 == 101.0 && mass2.0 == 500.0 {
            //     // info!("a {:?}", diff);
            //     info!("from {:?} to {:?} -- {:?}, {:?}", mass.0, mass2.0, diff, diff.length_squared());
            // }
            if let Some(mut force) = diff.try_normalize() {
                // if diff.length_squared() > 50.0 {
                    let magnitude = 1.0 * mass.0 * mass2.0 / diff.length_squared();
                    force *= magnitude;
                    let f = Vec2::new(force[0], force[1]);
                    acc.0 -= f;
                    acc2.0 += f;
                // }
            }
        }
        bodies.push((mass, trans, acc));
    }

    for (mass, _, acc) in bodies.iter_mut() {
        acc.0 /= mass.0;
    }
}

fn update_velocity(mut query: Query<(&mut Velocity, &Acceleration)>) {
    for (mut vel, acc) in query.iter_mut() {
        vel.0 += acc.0 * DT;
    }
}

fn movement(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, vel) in query.iter_mut() {
        transform.translation += Vec3::new(vel.0[0], vel.0[1], 0.0) * DT;
    }
}

fn cam(
    input_mouse: Res<Input<MouseButton>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    mut query: Query<(&mut Camera, &mut Transform, &GameCam)>
) {
    let pan_button = MouseButton::Left;

    let mut pan = Vec2::ZERO;
    let mut scroll = 0.0;

    if input_mouse.pressed(pan_button) {
        for ev in ev_motion.iter() {
            pan += ev.delta;
        }
    }

    for ev in ev_scroll.iter() {
        scroll -= ev.y * ZOOM_SENSITIVITY;
    }

    for (mut _cam, mut trans, _gamecam) in query.iter_mut() {
        if scroll.abs() > 0.0 {
            let new_scale = trans.scale + Vec3::new(scroll, scroll, 0.0);
            if new_scale[0] >= 1.0 && new_scale[0] <= 5.0 {
                trans.scale = new_scale;
            }
            // info!("{:?}", trans.scale);
        }

        if pan.length_squared() > 0.0 {
            let new_translation = Vec3::new(-pan.x * trans.scale[0], pan.y * trans.scale[0], 0.0);
            trans.translation += new_translation;
            // info!("{:?} {:?} {:?}", trans.translation, pan.x, pan.y);
        }

        // if input_mouse.pressed(pan_button) {
        //     trans.translation += Vec3::new(1.0, 0.0, 0.0);
        // }
    }
}

// TODO zoom in on mouse cursor
//  just get loc of cursor, set cam trans to that on zoom