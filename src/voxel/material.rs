use bevy::asset::Handle;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Sampler, ShaderRef};

use super::mesh::ATTRIBUTE_TEXTURE_INDEX;

#[derive(Resource, Deref)]
pub struct VoxelMaterialHandle(
    #[deref] pub Handle<ExtendedMaterial<StandardMaterial, VoxelMaterial>>,
);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct VoxelMaterial {
    #[texture(114, dimension = "2d_array")]
    #[sampler(115)]
    pub array_texture: Handle<Image>,

    #[uniform(116)]
    pub lod_offset: f32,
}

impl MaterialExtension for VoxelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/voxel_texture.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/voxel_texture.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        /* descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            /* Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(7), */
            ATTRIBUTE_TEXTURE_INDEX.at_shader_location(10),
        ])?]; */
        let vbl = layout
            .0
            .get_layout(&[ATTRIBUTE_TEXTURE_INDEX.at_shader_location(10)])?;

        descriptor
            .vertex
            .buffers
            .first_mut()
            .unwrap()
            .attributes
            .push(vbl.attributes[0]);
        Ok(())
    }
}
