use bevy::asset::{Asset, AssetLoader, AsyncReadExt};
use bevy::reflect::TypePath;
use thiserror::Error;

use super::LuaEngine;


#[derive(Asset, TypePath)]
pub struct LuaScript(pub mlua::Chunk<'static>);
pub struct OptionalLuaScript;

#[derive(Debug, Error)]
pub enum LuaScriptLoaderError {
    #[error("could not load file: {0}")]
    Io(#[from] std::io::Error),
}

pub struct LuaScriptLoader(pub LuaEngine);

impl AssetLoader for LuaScriptLoader {
    type Asset = LuaScript;

    type Settings = ();

    type Error = LuaScriptLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        async move {
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).await?;
            let chunk = self.0.load(buf);
            Ok(LuaScript(chunk))
        }
    }

    fn extensions(&self) -> &[&str] {
        &["lua"]
    }
}