#[test]
fn test_registry() {
    let chunk = mlua::chunk!(
        Registry:reg_block("grass", {
            textures = {
                top = "grass_carried"
            }
        });
    );
    let reg = minecrust::core::registry::Registry::new();
    let lua = mlua::Lua::new();
    //lua.globals().set("Registry", reg.clone()).unwrap();
    lua.load(chunk).exec().unwrap();
}
