#![allow(unused)]
#![warn(unused_mut, unused_must_use)]

use bevy::{input::common_conditions::input_just_pressed, prelude::*};

const ADDR: &str = "0.0.0.0:54345";

const DEFAULT_PORT: u16 = 54345;

mod server;
use server::ServerHandle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ServerHandle::start(ADDR))
        .insert_resource(DummyResource::new())
        .add_systems(
            Update,
            {
                |mut server: ResMut<ServerHandle>| {
                    if server.is_alive() {
                        server.shutdown();
                    }
                }
            }
            .run_if(input_just_pressed(KeyCode::Escape)),
        )
        .run();
}

#[derive(Resource)]
struct DummyResource;

impl DummyResource {
    fn new() -> Self {
        info!("initialized DummyResource");
        Self
    }
}
