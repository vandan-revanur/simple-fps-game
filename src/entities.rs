use cgmath::{InnerSpace, Vector3};

/// An enemy entity with a world-space position, alive state, and spawn angle.
#[derive(Clone)]
pub struct Enemy {
    pub position: Vector3<f32>,
    pub alive: bool,
    pub angle_degrees: i32,
}

/// A bullet projectile with position, direction, and remaining lifetime.
pub struct Bullet {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
    pub lifetime: f32,
}

/// Spawn the initial semicircle of enemies in front of the player.
///
/// Enemies are placed every 5° from −90° to +90° at a radius of 10 units and height 1.0.
pub fn spawn_enemies() -> Vec<Enemy> {
    let mut enemies = Vec::new();
    let distance = 10.0_f32;
    let height = 1.0_f32;

    for angle_deg in (-90..=90).step_by(5) {
        let angle_rad = (angle_deg as f32).to_radians();
        // Player starts looking down -Z, so the arc is in the XZ plane
        let x = angle_rad.sin() * distance;
        let z = -angle_rad.cos() * distance;

        enemies.push(Enemy {
            position: Vector3::new(x, height, z),
            alive: true,
            angle_degrees: angle_deg,
        });
    }

    enemies
}

/// Returns `true` when a bullet at `bullet_pos` is within the hit radius of `enemy_pos`.
pub fn check_bullet_hit(bullet_pos: Vector3<f32>, enemy_pos: Vector3<f32>) -> bool {
    (bullet_pos - enemy_pos).magnitude() < 1.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enemies_all_start_alive() {
        let enemies = spawn_enemies();
        for enemy in &enemies {
            assert!(enemy.alive, "all spawned enemies should start alive");
        }
    }

    #[test]
    fn test_enemy_spawn_count() {
        let enemies = spawn_enemies();
        // -90, -85, ..., 0, ..., 85, 90  →  (90 - (-90))/5 + 1 = 37
        assert_eq!(enemies.len(), 37, "should spawn 37 enemies");
    }

    #[test]
    fn test_enemy_spawn_height() {
        let enemies = spawn_enemies();
        for enemy in &enemies {
            assert!(
                (enemy.position.y - 1.0).abs() < 1e-5,
                "all enemies should be at height 1.0, got {}",
                enemy.position.y
            );
        }
    }

    #[test]
    fn test_enemy_spawn_radius() {
        let enemies = spawn_enemies();
        for enemy in &enemies {
            let xz_dist = (enemy.position.x.powi(2) + enemy.position.z.powi(2)).sqrt();
            assert!(
                (xz_dist - 10.0).abs() < 1e-4,
                "enemy at {}° should be 10 units from origin, got {}",
                enemy.angle_degrees,
                xz_dist
            );
        }
    }

    #[test]
    fn test_enemy_angle_range() {
        let enemies = spawn_enemies();
        let min_angle = enemies.iter().map(|e| e.angle_degrees).min().unwrap();
        let max_angle = enemies.iter().map(|e| e.angle_degrees).max().unwrap();
        assert_eq!(min_angle, -90, "minimum angle should be -90°");
        assert_eq!(max_angle, 90, "maximum angle should be 90°");
    }

    #[test]
    fn test_center_enemy_at_zero_degrees() {
        let enemies = spawn_enemies();
        let center = enemies.iter().find(|e| e.angle_degrees == 0).unwrap();
        // At 0°: x=sin(0)*10=0, z=-cos(0)*10=-10
        assert!(center.position.x.abs() < 1e-5, "center enemy x should be ~0");
        assert!((center.position.z + 10.0).abs() < 1e-4, "center enemy z should be ~-10");
    }

    #[test]
    fn test_bullet_hit_within_radius() {
        let enemy_pos = Vector3::new(0.0, 1.0, -10.0);
        let bullet_pos = Vector3::new(0.0, 1.0, -9.0); // 1.0 unit away
        assert!(check_bullet_hit(bullet_pos, enemy_pos), "bullet 1 unit away should hit");
    }

    #[test]
    fn test_bullet_miss_outside_radius() {
        let enemy_pos = Vector3::new(0.0, 1.0, -10.0);
        let bullet_pos = Vector3::new(5.0, 1.0, -10.0); // 5.0 units away
        assert!(!check_bullet_hit(bullet_pos, enemy_pos), "bullet 5 units away should miss");
    }

    #[test]
    fn test_bullet_hit_at_boundary() {
        let enemy_pos = Vector3::new(0.0, 0.0, 0.0);
        // exactly at boundary (1.5) — should NOT hit (strict less-than)
        let bullet_at_boundary = Vector3::new(1.5, 0.0, 0.0);
        assert!(
            !check_bullet_hit(bullet_at_boundary, enemy_pos),
            "bullet exactly at 1.5 should not hit (strict <)"
        );
        // just inside (1.4) — should hit
        let bullet_inside = Vector3::new(1.4, 0.0, 0.0);
        assert!(check_bullet_hit(bullet_inside, enemy_pos), "bullet at 1.4 should hit");
    }

    #[test]
    fn test_bullet_hit_exact_position() {
        let pos = Vector3::new(3.0, 1.0, -7.0);
        assert!(check_bullet_hit(pos, pos), "bullet at enemy position should always hit");
    }
}
