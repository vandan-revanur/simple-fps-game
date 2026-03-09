/// Keyboard and mouse-button input state.
pub struct Input {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub shoot: bool,
    pub look_left: bool,
    pub look_right: bool,
    pub look_up: bool,
    pub look_down: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            shoot: false,
            look_left: false,
            look_right: false,
            look_up: false,
            look_down: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_new_all_false() {
        let input = Input::new();
        assert!(!input.forward, "forward should start false");
        assert!(!input.backward, "backward should start false");
        assert!(!input.left, "left should start false");
        assert!(!input.right, "right should start false");
        assert!(!input.shoot, "shoot should start false");
        assert!(!input.look_left, "look_left should start false");
        assert!(!input.look_right, "look_right should start false");
        assert!(!input.look_up, "look_up should start false");
        assert!(!input.look_down, "look_down should start false");
    }

    #[test]
    fn test_input_fields_are_independently_mutable() {
        let mut input = Input::new();
        input.forward = true;
        assert!(input.forward);
        assert!(!input.backward, "other fields should remain false");
        input.shoot = true;
        assert!(input.forward);
        assert!(input.shoot);
        assert!(!input.left);
    }
}
