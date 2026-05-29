struct VSOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>) -> VSOut {
    var out: VSOut;
    out.clip_pos = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs(in: VSOut) -> @location(0) vec4<f32> {
    return in.color;
}
