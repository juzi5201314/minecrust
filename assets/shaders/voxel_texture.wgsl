#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_functions,
    skinning,
    pbr_types,
    view_transformations::position_world_to_clip
}
#import bevy_render::instance_index::get_instance_index

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_functions,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

#ifdef MESHLET_MESH_MATERIAL_PASS
#import bevy_pbr::meshlet_visibility_buffer_resolve::resolve_vertex_output
#endif

@group(2) @binding(114) var mat_array_texture: texture_2d_array<f32>;
@group(2) @binding(115) var mat_array_texture_sampler: sampler;
@group(2) @binding(116) var<uniform> lod_offset: f32;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(6) joint_indices: vec4<u32>,
    @location(7) joint_weights: vec4<f32>,
#endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif

    @location(10) texture_index: u32,
}

struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif

#ifdef MORPH_TARGETS
    @location(8) @interpolate(flat) vertex_index: u32,
#endif

    @location(10) texture_index: u32,
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[vertex.instance_index].first_vertex_index;
    let vertex_index = vertex.vertex_index - first_vertex;

    let weight_count = bevy_pbr::morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph(vertex_index, bevy_pbr::morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph(vertex_index, bevy_pbr::morph::normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph(vertex_index, bevy_pbr::morph::tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> CustomVertexOutput {
    var out: CustomVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var world_from_local = skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        model,
        vertex.tangent,
        vertex.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, world_from_local[3]);
#endif

    out.texture_index = vertex_no_morph.texture_index;
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    custom_in: CustomVertexOutput,
) -> FragmentOutput {
    var in: VertexOutput;
    in.position = custom_in.position;
    in.world_normal = custom_in.world_normal;
    in.world_position = custom_in.world_position;
    in.uv = custom_in.uv;
#ifdef VERTEX_UVS_B
    in.uv_b = custom_in.uv_b;
#endif
#ifdef VERTEX_COLORS
    in.color = custom_in.color;
#endif
    in.instance_index = custom_in.instance_index;
#ifdef VERTEX_TANGENTS
    in.world_tangent = custom_in.world_tangent;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    in.visibility_range_dither = custom_in.visibility_range_dither;
#endif

    //var texture_index = u32(custom_in.texture_index >> 4) + u32(custom_in.texture_index & 0xF);
    var texture_index = custom_in.texture_index;

#ifdef MESHLET_MESH_MATERIAL_PASS
    let in = resolve_vertex_output(frag_coord);
    let is_front = true;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    pbr_functions::visibility_range_dither(in.position, in.visibility_range_dither);
#endif

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let flag = true;

    if flag {
        // mip level
        let texture_size = vec2<f32>(vec2(textureDimensions(mat_array_texture, 0).xy));
        let base_lod = mip_level(in.uv, texture_size);
        let lod = clamp(base_lod + lod_offset, 0.0, 4.0);

        // texture sampling
        pbr_input.material.base_color = textureSampleLevel(
            mat_array_texture, mat_array_texture_sampler, 
            in.uv, 
            texture_index, 
            lod
        );
    } else {
        pbr_input.material.base_color = pixel_texture_array(in.uv, texture_index);
    }

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
    let out = deferred_output(in, pbr_input);
#else
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

/* bevy 0.15?
#ifdef OIT_ENABLED
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // The fragments will only be drawn during the oit resolve pass.
        bevy_core_pipeline::oit::oit_draw(in.position, out.color);
        discard;
    }
#endif // OIT_ENABLED
*/

    return out;
}

fn mip_level(uv: vec2<f32>, tex_size: vec2<f32>) -> f32 {
    let dx = dpdx(uv * tex_size[0]);
    let dy = dpdy(uv * tex_size[1]);
    let d = max(dot(dx, dx), dot(dy, dy));
    return 0.5 * log2(d);
}

// from https://www.reddit.com/r/VoxelGameDev/comments/1cxugd4/comment/l56zgod/?utm_source=share&utm_medium=web3x&utm_name=web3xcss&utm_term=1&utm_content=share_button
fn pixel_texture_array(uv: vec2<f32>, index: u32) -> vec4<f32> {
    // get texture size
    let texture_size: vec2<f32> = vec2<f32>(vec2(textureDimensions(mat_array_texture, 0).xy));
    let texel_size: vec4<f32> = vec4(1.0 / texture_size.x, 1.0 / texture_size.y, texture_size.x, texture_size.y);

    // box filter size in texel units
    let box_size: vec2<f32> = clamp(fwidth(uv.xy * texel_size.zw), vec2(1e-5), vec2(1.0));
    // scale uv by texture size to get texel coordinate
    let tx: vec2<f32> = uv.xy * texel_size.zw - 0.5 * box_size;
    // compute offset for pixel-sized box filter
    let tx_offset: vec2<f32> = smoothstep(1.0 - box_size, vec2(1.0), fract(tx));
    //vec2 tx_offset = clamp((fract(tx) - (1.0- box_size)) / box_size,0.0,1.0);

    // compute bilinear sample uv coordinates
    let bi_uv: vec2<f32> = (floor(tx) + 0.5 + tx_offset) * texel_size.xy;
    // sample the texture
    return textureSampleGrad(mat_array_texture, mat_array_texture_sampler, vec2(bi_uv), index, dpdx(uv.xy), dpdy(uv.xy));
}