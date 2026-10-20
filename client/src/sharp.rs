//! "Sharp bilinear" sampling of the images (not in the original, which only drew at 1:1): at
//! a zoom that is not a whole number of screen pixels per image pixel, plain nearest sampling
//! gives some image pixels one screen pixel more than others, a pattern that wobbles as the
//! map scrolls. Here every image pixel is a flat square of its colour, as with nearest, but
//! the screen pixel that straddles two of them blends them by how much of it each covers:
//! every image pixel keeps its exact width. At whole-number scales it is nearest sampling.
//!
//! One material draws the whole frame. Only the atlas's images ([`crate::assets::Sprite`]) use
//! it: their vertices carry, in `normal`, the image's rectangle on its page (sampling stays
//! inside it, never on the neighbouring image, frame or tile) and the scale they are drawn
//! at. Every other drawing has a zero `normal` and is sampled as macroquad does, only more
//! precisely (macroquad's own shader interpolates texture coordinates at low precision).

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams, UniformDesc, UniformType};
use macroquad::prelude::*;
use std::cell::OnceCell;

/// Side of the atlas pages, in pixels (`assets::PAGE`).
pub const PAGE: f32 = 4096.0;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;

varying vec4 color;
varying vec2 uv;
varying vec4 rect;
varying vec2 scale;

uniform mat4 Model;
uniform mat4 Projection;
uniform vec2 Viewport;

void main() {
    mat4 m = Projection * Model;
    gl_Position = m * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;
    if (normal.z > 0.0) {
        // `normal.xy`: left × 4096 + width − 1, and top × 4096 + height − 1, in page pixels
        vec2 start = floor(normal.xy / 4096.0);
        rect = vec4(start, start + normal.xy - start * 4096.0 + 1.0);
        // screen pixels per image pixel: per drawing unit, times `normal.zw` units per pixel
        scale = abs(vec2(m[0][0], m[1][1])) * Viewport * 0.5 * normal.zw;
    } else {
        rect = vec4(0.0);
        scale = vec2(0.0);
    }
}
"#;

const FRAGMENT: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

varying vec4 color;
varying vec2 uv;
varying vec4 rect;
varying vec2 scale;

uniform sampler2D Texture;

void main() {
    if (scale.x <= 0.0) {
        gl_FragColor = color * texture2D(Texture, uv);
        return;
    }
    vec2 p = uv * 4096.0;
    vec2 base = floor(p);
    // from the image pixel's centre; flat but for the last screen pixel to each side, which
    // moves to the edge shared with the next image pixel, where bilinear blends both halves
    // (zoomed out, `s` 1: plain bilinear)
    vec2 s = max(scale, vec2(1.0));
    vec2 d = p - base - 0.5;
    vec2 inner = 0.5 - 0.5 / s;
    p = base + 0.5 + (d - clamp(d, -inner, inner)) * s;
    p = clamp(p, rect.xy + 0.5, rect.zw - 0.5);
    gl_FragColor = color * texture2D(Texture, p / 4096.0);
}
"#;

thread_local! {
    static MATERIAL: OnceCell<Material> = const { OnceCell::new() };
}

/// Draws the frame from here with the material; `viewport` is the window's size in physical
/// pixels. Called at the start of every frame (macroquad goes back to its own each frame).
pub fn begin_frame(viewport: (f32, f32)) {
    MATERIAL.with(|m| {
        let material = m.get_or_init(|| {
            load_material(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                MaterialParams {
                    // blended like macroquad's own
                    pipeline_params: PipelineParams {
                        color_blend: Some(BlendState::new(
                            Equation::Add,
                            BlendFactor::Value(BlendValue::SourceAlpha),
                            BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                        )),
                        ..Default::default()
                    },
                    uniforms: vec![UniformDesc::new("Viewport", UniformType::Float2)],
                    ..Default::default()
                },
            )
            .unwrap_or_else(|e| panic!("sharp bilinear shader: {e}"))
        });
        material.set_uniform("Viewport", vec2(viewport.0, viewport.1));
        gl_use_material(material);
    });
}

/// `normal` for an image drawn from `source` (page pixels, whole) at `dest` (drawing units).
pub fn normal(source: Rect, dest: Vec2) -> Vec4 {
    let pack = |start: f32, len: f32| start.round() * PAGE + (len.round() - 1.0).max(0.0);
    vec4(pack(source.x, source.w), pack(source.y, source.h), dest.x / source.w, dest.y / source.h)
}
