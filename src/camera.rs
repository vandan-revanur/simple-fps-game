use cgmath::{perspective, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3};

use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};

/// First-person camera with position, yaw/pitch orientation, and projection parameters.
pub struct Camera {
    pub position: Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 1.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            aspect: WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
        }
    }

    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        let forward = Vector3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        );
        let target = self.position + forward;
        Matrix4::look_at_rh(
            Point3::from_vec(self.position),
            Point3::from_vec(target),
            Vector3::unit_y(),
        )
    }

    pub fn get_projection_matrix(&self) -> Matrix4<f32> {
        perspective(Deg(70.0), self.aspect, 0.1, 100.0)
    }

    pub fn get_view_projection(&self) -> Matrix4<f32> {
        self.get_projection_matrix() * self.get_view_matrix()
    }

    /// Returns the unit forward vector based on current yaw and pitch.
    pub fn get_forward_vector(&self) -> Vector3<f32> {
        Vector3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    /// Returns the right vector (perpendicular to forward, horizontal).
    pub fn get_right_vector(&self) -> Vector3<f32> {
        let forward = self.get_forward_vector().normalize();
        forward.cross(Vector3::unit_y()).normalize()
    }

    /// Returns the up vector relative to the camera orientation.
    pub fn get_up_vector(&self) -> Vector3<f32> {
        let forward = self.get_forward_vector().normalize();
        let right = self.get_right_vector();
        right.cross(forward).normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_new_defaults() {
        let cam = Camera::new();
        assert_eq!(cam.yaw, 0.0, "default yaw should be 0");
        assert_eq!(cam.pitch, 0.0, "default pitch should be 0");
        assert_eq!(cam.position, Vector3::new(0.0, 1.0, 0.0), "default position should be (0,1,0)");
    }

    #[test]
    fn test_forward_vector_at_zero_yaw_pitch() {
        let cam = Camera::new();
        let fwd = cam.get_forward_vector();
        // At yaw=0, pitch=0: (-sin(0)*cos(0), sin(0), -cos(0)*cos(0)) = (0, 0, -1)
        assert!(fwd.x.abs() < 1e-5, "x should be 0, got {}", fwd.x);
        assert!(fwd.y.abs() < 1e-5, "y should be 0, got {}", fwd.y);
        assert!((fwd.z + 1.0).abs() < 1e-5, "z should be -1, got {}", fwd.z);
    }

    #[test]
    fn test_right_vector_at_zero_yaw() {
        let cam = Camera::new();
        let right = cam.get_right_vector();
        // forward=(0,0,-1), right = forward × up_y = (0,0,-1) × (0,1,0) = (1,0,0)
        assert!((right.x - 1.0).abs() < 1e-5, "x should be 1, got {}", right.x);
        assert!(right.y.abs() < 1e-5, "y should be 0, got {}", right.y);
        assert!(right.z.abs() < 1e-5, "z should be 0, got {}", right.z);
    }

    #[test]
    fn test_up_vector_at_zero_orientation() {
        let cam = Camera::new();
        let up = cam.get_up_vector();
        // With forward=(0,0,-1) and right=(1,0,0): up = right × forward = (0,1,0)
        assert!(up.x.abs() < 1e-5, "x should be 0, got {}", up.x);
        assert!((up.y - 1.0).abs() < 1e-5, "y should be 1, got {}", up.y);
        assert!(up.z.abs() < 1e-5, "z should be 0, got {}", up.z);
    }

    #[test]
    fn test_default_aspect_ratio() {
        let cam = Camera::new();
        let expected = WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32;
        assert!((cam.aspect - expected).abs() < 1e-5, "aspect should match window dimensions");
    }

    #[test]
    fn test_pitch_up_gives_positive_y_forward() {
        let mut cam = Camera::new();
        cam.pitch = 0.5;
        let fwd = cam.get_forward_vector();
        assert!(fwd.y > 0.0, "pitching up should give positive y forward component");
    }

    #[test]
    fn test_yaw_90_degrees() {
        let mut cam = Camera::new();
        cam.yaw = std::f32::consts::FRAC_PI_2; // 90 degrees
        let fwd = cam.get_forward_vector();
        // At yaw=PI/2: (-sin(PI/2)*cos(0), 0, -cos(PI/2)*cos(0)) ≈ (-1, 0, 0)
        assert!((fwd.x + 1.0).abs() < 1e-5, "x should be -1 at yaw=90°, got {}", fwd.x);
        assert!(fwd.z.abs() < 1e-5, "z should be ~0 at yaw=90°, got {}", fwd.z);
    }

    #[test]
    fn test_view_projection_is_matrix4() {
        let cam = Camera::new();
        let vp = cam.get_view_projection();
        // Just check it produces a finite matrix
        let arr: [[f32; 4]; 4] = vp.into();
        for row in &arr {
            for val in row {
                assert!(val.is_finite(), "view-projection matrix should have finite values");
            }
        }
    }

    #[test]
    fn test_forward_vector_is_unit_length() {
        let mut cam = Camera::new();
        cam.yaw = 1.2;
        cam.pitch = 0.5;
        let fwd = cam.get_forward_vector();
        let len = fwd.magnitude();
        assert!((len - 1.0).abs() < 1e-5, "forward vector should be unit length, got {}", len);
    }
}
