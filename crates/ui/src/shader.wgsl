const vertices = array(
    vec2(-1., -1.),
    vec2(-1., 1.),
    vec2(1., -1.),
    vec2(1., 1.),
);

const tex_coords_arr = array(
    vec2(0., 0.),
    vec2(0., 1.),
    vec2(1., 0.),
    vec2(1., 1.),
);


struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@group(0)
@binding(3)
var<uniform> computed_delay_samples: f32;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let coordinates = vertices[in_vertex_index];
    out.tex_coords = tex_coords_arr[in_vertex_index];
    out.clip_position = vec4<f32>(coordinates, 0.0, 1.0);
    return out;
} 

@group(0) @binding(0)
var hor_text: texture_1d<f32>;
@group(0) @binding(1)
var ver_text: texture_1d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let hor = textureSample(hor_text, s_diffuse, in.tex_coords.x).r;
    let ver = textureSample(ver_text, s_diffuse, in.tex_coords.y).r;
    let difference = abs(ver - hor);
    var color = vec3(-log(difference) / 10.0);

    let on_delay_line = abs(in.tex_coords.x - computed_delay_samples) < 0.001;

    if on_delay_line {
        color.r = 1.0;
    }

    return vec4(color, 0.0);
}
 