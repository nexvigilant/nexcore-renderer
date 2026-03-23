// Fullscreen triangle blit shader.
//
// Copies Vello's Rgba8Unorm intermediate texture to the sRGB surface.
// Uses a single triangle with vertex_index trick (no vertex buffer needed).

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Fullscreen triangle: 3 vertices cover the entire clip space.
    let x = f32(i32(idx & 1u)) * 4.0 - 1.0;
    let y = f32(i32((idx >> 1u) & 1u)) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) / 2.0, (1.0 - y) / 2.0);
    return out;
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source_tex, source_sampler, in.uv);
}
