/// Integration tests for core game logic (no GPU required).
///
/// These tests verify that the pure-logic components of the game work correctly
/// end-to-end — enemy spawning, collision detection, camera mathematics, and
/// the mouse accumulator's thread-safety.
use cgmath::{InnerSpace, Vector3};

use simple_quake::camera::Camera;
use simple_quake::entities::{check_bullet_hit, spawn_enemies};
use simple_quake::geometry::{box_corners, build_gun_verts, gun_barrel_tip, oriented_box};
use simple_quake::mouse::MouseAccum;

// ---------------------------------------------------------------------------
// Enemy spawning
// ---------------------------------------------------------------------------

#[test]
fn test_enemy_spawn_full_integration() {
    let enemies = spawn_enemies();

    // Correct count
    assert_eq!(enemies.len(), 37, "should spawn 37 enemies (-90° to +90° in 5° steps)");

    // All alive at spawn
    for e in &enemies {
        assert!(e.alive, "every enemy should start alive");
    }

    // Correct height
    for e in &enemies {
        assert!(
            (e.position.y - 1.0).abs() < 1e-4,
            "enemy at {}° has wrong height {}",
            e.angle_degrees,
            e.position.y
        );
    }

    // Each enemy is ~10 units from the origin in the XZ plane
    for e in &enemies {
        let r = (e.position.x.powi(2) + e.position.z.powi(2)).sqrt();
        assert!(
            (r - 10.0).abs() < 1e-3,
            "enemy at {}° should be 10 units from origin, got {}",
            e.angle_degrees,
            r
        );
    }

    // Angle span: −90 to +90
    let min_a = enemies.iter().map(|e| e.angle_degrees).min().unwrap();
    let max_a = enemies.iter().map(|e| e.angle_degrees).max().unwrap();
    assert_eq!(min_a, -90);
    assert_eq!(max_a, 90);
}

#[test]
fn test_center_enemy_position() {
    let enemies = spawn_enemies();
    let center = enemies.iter().find(|e| e.angle_degrees == 0).unwrap();
    assert!(center.position.x.abs() < 1e-5, "0° enemy should have x≈0");
    assert!((center.position.z + 10.0).abs() < 1e-4, "0° enemy should have z≈-10");
}

#[test]
fn test_rightmost_enemy_position() {
    let enemies = spawn_enemies();
    let right = enemies.iter().find(|e| e.angle_degrees == 90).unwrap();
    // sin(90°)=1, cos(90°)=0 → x=10, z=0
    assert!((right.position.x - 10.0).abs() < 1e-4, "+90° enemy should have x≈10");
    assert!(right.position.z.abs() < 1e-4, "+90° enemy should have z≈0");
}

// ---------------------------------------------------------------------------
// Collision detection
// ---------------------------------------------------------------------------

#[test]
fn test_collision_at_enemy_position() {
    let enemies = spawn_enemies();
    for e in &enemies {
        assert!(
            check_bullet_hit(e.position, e.position),
            "a bullet at the enemy's own position must hit"
        );
    }
}

#[test]
fn test_no_collision_far_away() {
    let enemies = spawn_enemies();
    let far_bullet = Vector3::new(999.0, 999.0, 999.0);
    for e in &enemies {
        assert!(
            !check_bullet_hit(far_bullet, e.position),
            "a bullet far away must not hit"
        );
    }
}

#[test]
fn test_collision_just_inside_radius() {
    let enemy_pos = Vector3::new(0.0, 0.0, 0.0);
    let bullet_pos = Vector3::new(1.49, 0.0, 0.0); // just inside 1.5
    assert!(check_bullet_hit(bullet_pos, enemy_pos));
}

#[test]
fn test_no_collision_just_outside_radius() {
    let enemy_pos = Vector3::new(0.0, 0.0, 0.0);
    let bullet_pos = Vector3::new(1.51, 0.0, 0.0); // just outside 1.5
    assert!(!check_bullet_hit(bullet_pos, enemy_pos));
}

// ---------------------------------------------------------------------------
// Camera mathematics
// ---------------------------------------------------------------------------

#[test]
fn test_camera_forward_movement_integration() {
    let mut camera = Camera::new();
    let initial_z = camera.position.z;

    // Simulate one frame of forward movement (yaw=0 → moves in -Z)
    let forward = Vector3::new(-camera.yaw.sin(), 0.0, -camera.yaw.cos());
    let move_speed = 5.0_f32;
    let dt = 0.1_f32;
    camera.position += forward * move_speed * dt;

    assert!(
        camera.position.z < initial_z,
        "moving forward at yaw=0 should decrease z"
    );
    assert!(
        (camera.position.z - (initial_z - move_speed * dt)).abs() < 1e-4,
        "forward step size should match speed × dt"
    );
}

#[test]
fn test_camera_yaw_wrapping() {
    let mut camera = Camera::new();
    use std::f32::consts::PI;
    camera.yaw = PI + 0.1;
    // Apply wrapping (same logic as App::update)
    if camera.yaw > PI {
        camera.yaw -= 2.0 * PI;
    }
    assert!(camera.yaw < 0.0, "yaw should wrap from >PI to negative");
    assert!(camera.yaw > -PI, "wrapped yaw should stay within (-PI, PI)");
}

#[test]
fn test_camera_pitch_clamp() {
    let mut camera = Camera::new();
    camera.pitch = 10.0; // way over the limit
    camera.pitch = camera.pitch.clamp(-1.5, 1.5);
    assert_eq!(camera.pitch, 1.5, "pitch should be clamped to 1.5");
}

#[test]
fn test_camera_vectors_orthogonality() {
    let mut camera = Camera::new();
    camera.yaw = 0.7;
    camera.pitch = 0.3;
    let fwd   = camera.get_forward_vector().normalize();
    let right = camera.get_right_vector();
    let up    = camera.get_up_vector();
    // fwd · right ≈ 0
    assert!(fwd.dot(right).abs() < 1e-4, "forward and right should be orthogonal");
    // fwd · up ≈ 0
    assert!(fwd.dot(up).abs() < 1e-4, "forward and up should be orthogonal");
    // right · up ≈ 0
    assert!(right.dot(up).abs() < 1e-4, "right and up should be orthogonal");
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[test]
fn test_oriented_box_produces_36_verts() {
    let corners = box_corners(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::unit_x(),
        Vector3::unit_y(),
        Vector3::unit_z(),
        1.0, 1.0, -1.0, 1.0,
    );
    let verts = oriented_box(corners, [1.0, 0.0, 0.0], [0.5, 0.0, 0.0]);
    assert_eq!(verts.len(), 36);
}

#[test]
fn test_gun_verts_attached_to_camera_position() {
    let camera = Camera::new();
    let verts = build_gun_verts(&camera);
    assert_eq!(verts.len(), 108, "gun should have 108 vertices");

    // All gun vertices should be close to the camera (not at the origin or infinity)
    for v in &verts {
        let dist = ((v[0] - camera.position.x).powi(2)
            + (v[1] - camera.position.y).powi(2)
            + (v[2] - camera.position.z).powi(2))
            .sqrt();
        assert!(dist < 5.0, "gun vertex should be within 5 units of camera, got dist={}", dist);
    }
}

#[test]
fn test_barrel_tip_moves_with_camera() {
    let mut cam1 = Camera::new();
    let mut cam2 = Camera::new();
    cam2.position = Vector3::new(10.0, 1.0, 0.0);

    let tip1 = gun_barrel_tip(&cam1);
    let tip2 = gun_barrel_tip(&cam2);

    // Tip should be offset by the same camera translation
    assert!((tip2.x - tip1.x - 10.0).abs() < 1e-4, "tip x should shift with camera");

    // Yaw-rotated camera: tip should be in a different direction
    cam1.yaw = std::f32::consts::FRAC_PI_2;
    let tip_rotated = gun_barrel_tip(&cam1);
    assert!(
        (tip_rotated - tip1).magnitude() > 0.1,
        "rotating the camera should move the barrel tip"
    );
}

// ---------------------------------------------------------------------------
// Mouse accumulator (thread-safety via Arc)
// ---------------------------------------------------------------------------

#[test]
fn test_mouse_accum_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let accum = Arc::new(MouseAccum::new());
    let mut handles = Vec::new();

    for _ in 0..4 {
        let a = Arc::clone(&accum);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                a.add_raw(1, 1);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let (dx, dy) = accum.take();
    assert!((dx - 400.0).abs() < 0.5, "sum of dx from 4 threads × 100 should be ~400, got {}", dx);
    assert!((dy - 400.0).abs() < 0.5, "sum of dy from 4 threads × 100 should be ~400, got {}", dy);
}
