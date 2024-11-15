use bevy::app::Plugin;
use bevy::prelude::*;
use lua_script_loader::LuaScriptLoader;
use mlua::{Compiler, Lua, LuaOptions};

pub use lua_script_loader::LuaScript;

mod lua_script_loader;

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let lua = unsafe {
            Lua::unsafe_new_with(
                mlua::StdLib::ALL,
                LuaOptions::new().thread_pool_size((num_cpus::get() / 8).max(1)),
            )
        };
        lua.set_compiler(
            Compiler::new().set_optimization_level(if cfg!(debug_assertions) { 1 } else { 2 }),
        );
        let lua = LuaEngine(lua);
        app.init_asset::<LuaScript>()
            .register_asset_loader(LuaScriptLoader(lua.clone()))
            .insert_resource(lua);
    }
}

#[derive(Resource, Deref, DerefMut, Clone)]
pub struct LuaEngine(#[deref] pub mlua::Lua);
