use std::sync::Arc;

use ahash::AHashMap;
use basis_universal::ColorSpace;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageFormat, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use bevy_asset_loader::asset_collection::AssetCollection;

use crate::voxel::textures::TextureMap;

#[derive(AssetCollection, Resource)]
pub struct BlockTextureAssets {
    #[asset(path = "textures/blocks", collection(typed, mapped))]
    blocks: bevy::utils::hashbrown::HashMap<String, Handle<Image>>,
}

#[derive(Resource)]
pub struct VoxelTextures(pub Handle<Image>);

pub fn load_textures(
    mut commands: Commands,
    mut image_assets: ResMut<Assets<Image>>,
    textures: Res<BlockTextureAssets>,
    device: Option<Res<RenderDevice>>,
) {
    const IMAGE_SIZE: u32 = 16;

    let time = std::time::Instant::now();
    let mut texture_map = AHashMap::with_capacity(textures.blocks.len());

    //TODO: temp solution
    const CACHE: &str = "assets/cache/textures.basis";
    let bytes = if let Ok(bytes) = std::fs::read(CACHE) {
        bytes
    } else {
        std::fs::create_dir_all("assets/cache").ok();

        let images = textures.blocks.iter().filter_map(|(path, h)| {
            let image = image_assets.get(h).unwrap();
            if image.width() != IMAGE_SIZE || image.height() != IMAGE_SIZE {
                None
            } else {
                Some((path, image))
            }
        });

        let mut compressor_params = basis_universal::CompressorParams::new();
        compressor_params.set_basis_format(basis_universal::BasisTextureFormat::UASTC4x4);
        compressor_params.set_generate_mipmaps(true);
        compressor_params.set_color_space(ColorSpace::Srgb);
        compressor_params.set_uastc_quality_level(basis_universal::UASTC_QUALITY_DEFAULT);

        for (i, (path, image)) in images.enumerate() {
            let mut source_image = compressor_params.source_image_mut(i as u32);
            let size = image.size();
            source_image.init(&image.data, size.x, size.y, 4);
            texture_map.insert(path.clone(), i);
        }

        let mut compressor = basis_universal::Compressor::new(16);
        // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
        // library bindings note that invalid params might produce undefined behavior.
        unsafe {
            compressor.init(&compressor_params);
            compressor.process().unwrap();
        }
        let b = compressor.basis_file().to_vec();
        //std::fs::write(CACHE, &b).unwrap();
        b
    };

    let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..Default::default()
    });

    let image = Image::from_buffer(
        &bytes,
        ImageType::Format(ImageFormat::Basis),
        device
            .map(|device| device.features())
            .map(CompressedImageFormats::from_features)
            .unwrap_or_else(|| CompressedImageFormats::NONE),
        true,
        sampler,
        RenderAssetUsages::default(),
    )
    .unwrap();

    info!(
        "loaded {} textures ({} bytes), took {:?}",
        image.texture_descriptor.array_layer_count(),
        image.data.len(),
        time.elapsed()
    );

    commands.insert_resource(TextureMap(Arc::new(texture_map)));
    commands.insert_resource(VoxelTextures(image_assets.add(image)));
}

pub fn unload_textures(mut commands: Commands) {
    commands.remove_resource::<BlockTextureAssets>();
}

/* pub fn load_textures(
    mut commands: Commands,
    mut image_assets: ResMut<Assets<Image>>,
    textures: Res<BlockTextureAssets>,
    device: Option<Res<RenderDevice>>,
) {
    const IMAGE_SIZE: u32 = 16;

    let mut pixels = Vec::new();
    let time = std::time::Instant::now();
    let f = device.map(|device| device.features());

    let image = image_assets.get(&textures.blocks[0]).unwrap();
    let is_srgb = true;
    let compressed_basis_data = {
        let mut compressor_params = basis_universal::CompressorParams::new();
        compressor_params.set_basis_format(basis_universal::BasisTextureFormat::UASTC4x4);
        compressor_params.set_generate_mipmaps(true);
        let color_space = if is_srgb {
            basis_universal::ColorSpace::Srgb
        } else {
            basis_universal::ColorSpace::Linear
        };
        compressor_params.set_color_space(color_space);
        compressor_params.set_uastc_quality_level(basis_universal::UASTC_QUALITY_DEFAULT);

        for (i, image) in textures
            .blocks
            .iter()
            .filter_map(|h| {
                let image = image_assets.get(h).unwrap();
                if image.width() != IMAGE_SIZE || image.height() != IMAGE_SIZE {
                    None
                } else {
                    Some(image.clone())
                }
            })
            .enumerate()
        {
            let mut source_image = compressor_params.source_image_mut(i as u32);
            let size = image.size();

            source_image.init(&image.data, size.x, size.y, 4);
        }

        let mut compressor = basis_universal::Compressor::new(5);
        // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
        // library bindings note that invalid params might produce undefined behavior.
        unsafe {
            compressor.init(&compressor_params);
            compressor.process().unwrap();
        }
        compressor.basis_file().to_vec()
    };

    let bytes = Image::from_buffer(
        &compressed_basis_data,
        ImageType::Format(ImageFormat::Basis),
        f.map(CompressedImageFormats::from_features)
            .unwrap_or_else(|| CompressedImageFormats::NONE),
        is_srgb,
        image.sampler.clone(),
        image.asset_usage,
    )
    .unwrap();
    dbg!(bytes.texture_descriptor);

    for handle in textures.blocks.iter() {
        let image = image_assets
            .get(handle)
            .unwrap()
            .clone()
            .try_into_dynamic()
            .unwrap();
        let bytes = image.as_bytes();
        //let bytes = &image_assets.get(handle).unwrap().data;
        if image.width() != IMAGE_SIZE || image.height() != IMAGE_SIZE {
            continue;
        }
        pixels.extend_from_slice(bytes);
    }

    let image = Image::new_fill(
        Extent3d {
            width: IMAGE_SIZE,
            height: IMAGE_SIZE,
            depth_or_array_layers: textures.blocks.len() as u32,
        },
        TextureDimension::D2,
        &pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.clone().try_into_dynamic().unwrap().save("a.png");

    //image.reinterpret_stacked_2d_as_array(textures.blocks.len() as u32);

    info!(
        "loaded {} textures ({} bytes), took {:?}",
        textures.blocks.len(),
        image.data.len(),
        time.elapsed()
    );

    commands.insert_resource(VoxelTextures(image_assets.add(image)));
}
 */
