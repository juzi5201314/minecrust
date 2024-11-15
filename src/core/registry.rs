use std::sync::Arc;

use crate::atom::Atom;
use crate::script::{LuaEngine, LuaScript};
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use bevy_asset_loader::asset_collection::AssetCollection;
use block::{BlockMetadata, BlockRegistry};
use mlua::{LuaSerdeExt, Table, UserData};

pub mod block;

#[derive(Resource, AssetCollection)]
pub struct RegistryAssets {
    #[asset(path = "registries", collection(typed, mapped))]
    registries: HashMap<String, Handle<LuaScript>>,
}

#[derive(Debug, Default, Clone, Resource)]
pub struct Registry {
    blocks: Arc<papaya::HashMap<Atom, BlockRegistry, ahash::RandomState>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            ..Default::default()
        }
    }

    pub fn get_block_cloned(&self, id: &str) -> Option<BlockRegistry> {
        self.blocks.pin().get(id).map(Clone::clone)
    }

    #[inline]
    pub fn get_block_with<T>(&self, id: &Atom, f: impl FnOnce(&BlockRegistry) -> T) -> Option<T> {
        self.blocks.pin().get(id).map(f)
    }
}

impl UserData for Registry {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut::<_, (String, Table), _>("set_block", |lua, this, (id, table)| {
            let metadata = Arc::new(lua.from_value::<BlockMetadata>(mlua::Value::Table(table))?);
            let namespace = lua
                .globals()
                .get::<String>("namespace")
                .unwrap_or_else(|_| "unknown".to_owned());
            let id = Atom::new(format!("{namespace}::{id}"));
            let registry = BlockRegistry {
                id: id.clone(),
                metadata,
            };
            this.blocks.pin().insert(id, registry);
            Ok(())
        });
    }
}

pub fn register_core_items(
    lua: Res<LuaEngine>,
    registry: Res<Registry>,
    registries: Res<RegistryAssets>,
    mut assets: ResMut<Assets<LuaScript>>,
) {
    lua.globals().set("Registry", registry.clone()).unwrap();
    lua.sandbox(true).unwrap();
    lua.globals().set("namespace", "core").unwrap();

    for (name, script_handle) in &registries.registries {
        let script = assets.remove(script_handle).unwrap();
        script.0.exec().unwrap();
        tracing::info!("Loaded registry: {}", name);
    }
    lua.sandbox(false).unwrap();
}
