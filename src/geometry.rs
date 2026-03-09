use cgmath::{InnerSpace, Vector3};

use crate::camera::Camera;

/// Emit 36 vertices for an oriented box defined by its 8 corners in world space.
///
/// Corner layout (r = right axis, u = up axis, f = forward axis):
///   0: (-r, -u, -f)   "back-bottom-left"
///   1: (+r, -u, -f)   "back-bottom-right"
///   2: (+r, +u, -f)   "back-top-right"
///   3: (-r, +u, -f)   "back-top-left"
///   4: (-r, -u, +f)   "front-bottom-left"
///   5: (+r, -u, +f)   "front-bottom-right"
///   6: (+r, +u, +f)   "front-top-right"
///   7: (-r, +u, +f)   "front-top-left"
pub fn oriented_box(corners: [Vector3<f32>; 8], color: [f32; 3], dark: [f32; 3]) -> Vec<[f32; 6]> {
    let c = color;
    let d = dark;
    let v = |p: Vector3<f32>, col: [f32; 3]| -> [f32; 6] {
        [p.x, p.y, p.z, col[0], col[1], col[2]]
    };
    let [c0, c1, c2, c3, c4, c5, c6, c7] = corners;
    // shade each face slightly differently for visual depth cues
    let front  = c;
    let back   = d;
    let top    = [c[0] * 0.8, c[1] * 0.8, c[2] * 0.8];
    let bottom = [d[0] * 0.6, d[1] * 0.6, d[2] * 0.6];
    let right  = [c[0] * 0.7, c[1] * 0.7, c[2] * 0.7];
    let left   = [c[0] * 0.9, c[1] * 0.9, c[2] * 0.9];
    vec![
        // Front face  (c4 c5 c6 c7)
        v(c4, front), v(c5, front), v(c6, front),
        v(c4, front), v(c6, front), v(c7, front),
        // Back face   (c1 c0 c3 c2)
        v(c1, back),  v(c0, back),  v(c3, back),
        v(c1, back),  v(c3, back),  v(c2, back),
        // Top face    (c3 c2 c6 c7)
        v(c3, top),   v(c2, top),   v(c6, top),
        v(c3, top),   v(c6, top),   v(c7, top),
        // Bottom face (c0 c1 c5 c4)
        v(c0, bottom), v(c1, bottom), v(c5, bottom),
        v(c0, bottom), v(c5, bottom), v(c4, bottom),
        // Right face  (c1 c2 c6 c5)
        v(c1, right), v(c2, right), v(c6, right),
        v(c1, right), v(c6, right), v(c5, right),
        // Left face   (c0 c4 c7 c3)
        v(c0, left),  v(c4, left),  v(c7, left),
        v(c0, left),  v(c7, left),  v(c3, left),
    ]
}

/// Build oriented-box corners for a box whose axes are (right, up, fwd) in world space,
/// spanning `[-hw..+hw]` along right, `[-hh..+hh]` along up, `[f0..f1]` along fwd,
/// all relative to `anchor`.
pub fn box_corners(
    anchor: Vector3<f32>,
    right: Vector3<f32>,
    up: Vector3<f32>,
    fwd: Vector3<f32>,
    hw: f32,
    hh: f32,
    f0: f32,
    f1: f32,
) -> [Vector3<f32>; 8] {
    let p = |r: f32, u: f32, f: f32| anchor + right * r + up * u + fwd * f;
    [
        p(-hw, -hh, f0), // 0 back-bottom-left
        p( hw, -hh, f0), // 1 back-bottom-right
        p( hw,  hh, f0), // 2 back-top-right
        p(-hw,  hh, f0), // 3 back-top-left
        p(-hw, -hh, f1), // 4 front-bottom-left
        p( hw, -hh, f1), // 5 front-bottom-right
        p( hw,  hh, f1), // 6 front-top-right
        p(-hw,  hh, f1), // 7 front-top-left
    ]
}

/// Build 3D gun vertices positioned in world space relative to the camera,
/// so it always appears in the lower-right of the screen like a classic FPS.
pub fn build_gun_verts(camera: &Camera) -> Vec<[f32; 6]> {
    let fwd   = camera.get_forward_vector().normalize();
    let right = camera.get_right_vector();
    let up    = camera.get_up_vector();

    // Gun anchor: slightly in front, to the right, and below the camera
    let anchor = camera.position
        + fwd   *  0.6
        + right *  0.35
        + up    * -0.28;

    let barrel_hw = 0.03_f32;
    let barrel_hh = 0.03_f32;

    let body_hw  = 0.06_f32;
    let body_hh  = 0.08_f32;
    let body_len = 0.20_f32;

    let grip_hw     = 0.04_f32;
    let grip_height = 0.12_f32;
    let grip_len    = 0.08_f32;

    let barrel_col : [f32; 3] = [0.25, 0.25, 0.25];
    let barrel_dark: [f32; 3] = [0.15, 0.15, 0.15];
    let body_col   : [f32; 3] = [0.35, 0.30, 0.20];
    let body_dark  : [f32; 3] = [0.20, 0.17, 0.10];
    let grip_col   : [f32; 3] = [0.20, 0.15, 0.10];
    let grip_dark  : [f32; 3] = [0.12, 0.09, 0.06];

    let barrel_corners = box_corners(anchor, right, up, fwd, barrel_hw, barrel_hh, 0.0, 0.55);
    let body_corners   = box_corners(anchor, right, up, fwd, body_hw,   body_hh,   -body_len, 0.0);

    let grip_anchor  = anchor + up * (-body_hh);
    let grip_corners = box_corners(
        grip_anchor, right, up, fwd,
        grip_hw, grip_height * 0.5,
        -body_len + grip_len * 2.0, -body_len + grip_len * 4.0,
    );

    let mut verts = Vec::new();
    verts.extend(oriented_box(barrel_corners, barrel_col, barrel_dark));
    verts.extend(oriented_box(body_corners,   body_col,   body_dark));
    verts.extend(oriented_box(grip_corners,   grip_col,   grip_dark));
    verts
}

/// Returns the world-space position of the barrel muzzle (uses the same offsets as
/// `build_gun_verts`).
pub fn gun_barrel_tip(camera: &Camera) -> Vector3<f32> {
    let fwd   = camera.get_forward_vector().normalize();
    let right = camera.get_right_vector();
    let up    = camera.get_up_vector();
    let anchor = camera.position
        + fwd   *  0.6
        + right *  0.35
        + up    * -0.28;
    // barrel tip = anchor + fwd * 0.55 (the front face of the barrel box)
    anchor + fwd * 0.55
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oriented_box_vertex_count() {
        let corners = box_corners(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::unit_x(),
            Vector3::unit_y(),
            Vector3::unit_z(),
            1.0, 1.0, -1.0, 1.0,
        );
        let verts = oriented_box(corners, [1.0, 0.0, 0.0], [0.5, 0.0, 0.0]);
        assert_eq!(verts.len(), 36, "6 faces × 2 triangles × 3 verts = 36");
    }

    #[test]
    fn test_box_corners_count() {
        let corners = box_corners(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::unit_x(),
            Vector3::unit_y(),
            Vector3::unit_z(),
            1.0, 1.0, -1.0, 1.0,
        );
        assert_eq!(corners.len(), 8, "box_corners should return exactly 8 corners");
    }

    #[test]
    fn test_box_corners_positions_axis_aligned() {
        let anchor = Vector3::new(0.0, 0.0, 0.0);
        let corners = box_corners(
            anchor,
            Vector3::unit_x(),
            Vector3::unit_y(),
            Vector3::unit_z(),
            1.0, 1.0, 0.0, 2.0,
        );
        // corner 0: back-bottom-left: (-1, -1, 0)
        assert!((corners[0] - Vector3::new(-1.0, -1.0, 0.0)).magnitude() < 1e-5);
        // corner 6: front-top-right: (+1, +1, 2)
        assert!((corners[6] - Vector3::new(1.0, 1.0, 2.0)).magnitude() < 1e-5);
    }

    #[test]
    fn test_box_corners_with_offset_anchor() {
        let anchor = Vector3::new(5.0, 0.0, 0.0);
        let corners = box_corners(
            anchor,
            Vector3::unit_x(),
            Vector3::unit_y(),
            Vector3::unit_z(),
            1.0, 1.0, 0.0, 1.0,
        );
        // all corners should have x in [4.0, 6.0]
        for corner in &corners {
            assert!(corner.x >= 3.9 && corner.x <= 6.1, "x should be near 5±1, got {}", corner.x);
        }
    }

    #[test]
    fn test_build_gun_verts_count() {
        let camera = Camera::new();
        let verts = build_gun_verts(&camera);
        // 3 parts (barrel, body, grip) × 36 verts each = 108
        assert_eq!(verts.len(), 108, "gun should have 108 vertices (3 parts × 36)");
    }

    #[test]
    fn test_gun_barrel_tip_in_front_of_camera() {
        let camera = Camera::new();
        let tip = gun_barrel_tip(&camera);
        // Camera at (0,1,0) looking in -Z direction; tip should have z < camera z
        assert!(tip.z < camera.position.z, "barrel tip should be in front of the camera");
    }

    #[test]
    fn test_gun_barrel_tip_to_right_of_camera() {
        let camera = Camera::new();
        let tip = gun_barrel_tip(&camera);
        // At yaw=0 the right vector is +X, so tip.x > camera.x
        assert!(tip.x > camera.position.x, "barrel tip should be to the right of camera");
    }

    #[test]
    fn test_oriented_box_vertex_format() {
        let corners = box_corners(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::unit_x(), Vector3::unit_y(), Vector3::unit_z(),
            1.0, 1.0, 0.0, 1.0,
        );
        let verts = oriented_box(corners, [1.0, 0.0, 0.0], [0.5, 0.0, 0.0]);
        for v in &verts {
            assert_eq!(v.len(), 6, "each vertex should have 6 floats (xyz + rgb)");
            // All values should be finite
            for val in v {
                assert!(val.is_finite(), "vertex value should be finite");
            }
        }
    }
}
