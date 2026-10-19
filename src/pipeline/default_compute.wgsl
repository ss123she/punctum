@group(0) @binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(output_texture);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }
    let uv = vec2<f32>(global_id.xy) / vec2<f32>(dims);
    let color = vec4<f32>(uv.x, uv.y, 0.5, 1.0);
    textureStore(output_texture, global_id.xy, color);
}
