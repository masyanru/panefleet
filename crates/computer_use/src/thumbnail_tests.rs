//! Unit tests for the pure-Rust thumbnail compositing. These exercise the
//! play-button glyph rendering and blending without ffmpeg, so they are
//! deterministic and run anywhere the `image` crate is available.

use std::path::PathBuf;

use image::{Rgba, RgbaImage};

use super::*;

/// A solid-color frame keeps its corners untouched while the centered glyph
/// darkens the disc and turns the triangle white.
#[test]
fn composite_play_button_darkens_disc_and_whites_triangle() {
    let mut frame = solid_frame(100, 100, [200, 0, 0]);
    composite_play_button(&mut frame);

    // A corner is outside the glyph and stays the original red.
    assert_eq!(
        frame.get_pixel(0, 0),
        &Rgba([200, 0, 0, 255]),
        "corner pixel should be untouched"
    );

    // The frame center lands inside the triangle, so it becomes white.
    let center = frame.get_pixel(50, 50);
    assert!(
        center.0[1] > 200 && center.0[2] > 200,
        "center pixel should be the white triangle, got {:?}",
        center
    );

    // A point inside the disc but above the triangle is darkened toward black
    // (the semi-transparent disc blended over red), not whitened.
    let disc_only = frame.get_pixel(50, 30);
    assert!(
        disc_only.0[0] < 200 && disc_only.0[1] == 0 && disc_only.0[2] == 0,
        "disc-only pixel should be darkened red, got {:?}",
        disc_only
    );
}

/// The play-button overlay is transparent outside the shape and non-transparent
/// inside it, with the triangle rendered white and the rest of the disc dark.
#[test]
fn play_button_overlay_shape_and_colors() {
    let overlay = render_play_button_overlay(200, 200);
    // diameter = max(0.22 * 200, 48) = 48
    assert_eq!(overlay.dimensions(), (48, 48));

    // A corner of the overlay is outside the disc: fully transparent.
    assert_eq!(
        overlay.get_pixel(0, 0),
        &Rgba([0, 0, 0, 0]),
        "overlay corner should be transparent"
    );

    // The center is inside the triangle: white with full coverage.
    let center = overlay.get_pixel(24, 24);
    assert!(
        center.0[3] > 0 && center.0[0] > 200 && center.0[1] > 200 && center.0[2] > 200,
        "overlay center should be the white triangle, got {:?}",
        center
    );

    // A disc-only point (top of the disc, above the triangle) is dark with
    // partial alpha, never white.
    let disc_only = overlay.get_pixel(24, 3);
    assert!(
        disc_only.0[3] > 0,
        "disc-only pixel should be non-transparent, got {:?}",
        disc_only
    );
    assert!(
        disc_only.0[0] < 50 && disc_only.0[1] < 50 && disc_only.0[2] < 50,
        "disc-only pixel should be dark, got {:?}",
        disc_only
    );
}

/// The disc diameter scales with the smaller frame dimension but never drops
/// below the visibility floor.
#[test]
fn play_button_diameter_scales_with_floor() {
    // 22% of the smaller edge, rounded.
    assert_eq!(play_button_diameter(1280, 720), 158); // 0.22 * 720 = 158.4 -> 158
    // Below the floor, the minimum kicks in.
    assert_eq!(play_button_diameter(50, 50), 48);
}

/// The thumbnail path gets a distinct `.thumb.png` suffix that cannot collide
/// with the video's filepath.
#[test]
fn thumbnail_path_appends_suffix() {
    let video = PathBuf::from("/tmp/warp-recording-abc.mp4");
    assert_eq!(
        thumbnail_path(&video),
        PathBuf::from("/tmp/warp-recording-abc.mp4.thumb.png")
    );
}

/// Generating a thumbnail for a nonexistent video fails gracefully rather than
/// panicking, regardless of whether ffmpeg is installed.
#[tokio::test]
async fn generate_thumbnail_for_missing_video_errors() {
    let result = generate_video_thumbnail(
        &PathBuf::from("/nonexistent/warp-recording-missing.mp4"),
        DEFAULT_THUMBNAIL_MAX_WIDTH,
    )
    .await;
    assert!(
        result.is_err(),
        "missing video should produce a thumbnail error, got {result:?}"
    );
}

fn solid_frame(width: u32, height: u32, rgb: [u8; 3]) -> RgbaImage {
    let mut frame = RgbaImage::new(width, height);
    for pixel in frame.pixels_mut() {
        *pixel = Rgba([rgb[0], rgb[1], rgb[2], 255]);
    }
    frame
}
