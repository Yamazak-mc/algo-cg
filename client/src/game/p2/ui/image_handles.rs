use bevy::{prelude::*, utils::HashMap};
use std::borrow::Cow;

#[derive(Default, Resource)]
pub struct ImageHandles(HashMap<Cow<'static, str>, Option<Handle<Image>>>);

impl ImageHandles {
    pub fn new<K: Into<Cow<'static, str>>>(keys: impl IntoIterator<Item = K>) -> Self {
        let inner = keys.into_iter().map(|k| (k.into(), None)).collect();
        Self(inner)
    }

    pub fn load(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        asset_server: &AssetServer,
    ) -> Handle<Image> {
        let key = key.into();
        let val = self.0.get_mut(&key).unwrap();
        match val {
            Some(handle) => handle.clone(),
            None => {
                let handle = asset_server.load(&*key);
                *val = Some(handle.clone());
                handle
            }
        }
    }
}
