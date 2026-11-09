/// Independent right-click latches. A held mouse button toggles at most once.
pub struct Latches<const N: usize> {
    held: [bool; N],
    right_down: bool,
}

impl<const N: usize> Latches<N> {
    pub fn new() -> Self { Self { held: [false; N], right_down: false } }

    pub fn update(&mut self, right_down: bool, id: Option<usize>) {
        if right_down && !self.right_down {
            if let Some(id) = id.filter(|&id| id < N) {
                self.held[id] = !self.held[id];
            }
        }
        self.right_down = right_down;
    }

    pub fn is_held(&self, id: usize) -> bool { self.held[id] }

    pub fn merge(&self, want: &mut [bool]) {
        for (wanted, held) in want.iter_mut().zip(self.held.iter()) {
            *wanted |= *held;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latched_and_momentary_inputs_combine_without_cancelling() {
        let mut latch = Latches::<4>::new();
        latch.update(true, Some(1));
        let mut momentary = [false, true, true, false];
        latch.merge(&mut momentary);
        assert_eq!(momentary, [false, true, true, false]);
        let mut released = [false; 4];
        latch.merge(&mut released);
        assert_eq!(released, [false, true, false, false]);
        latch.update(false, None);
        latch.update(true, Some(1));
        let mut still_pressed = [false, true, false, false];
        latch.merge(&mut still_pressed);
        assert!(still_pressed[1]);
    }

    #[test]
    fn independent_controls_toggle_once_per_right_click() {
        let mut latch = Latches::<80>::new();
        latch.update(true, Some(3));
        latch.update(true, Some(3)); // holding the mouse button must not toggle repeatedly
        latch.update(false, None);
        latch.update(true, Some(7));
        assert!(latch.is_held(3));
        assert!(latch.is_held(7));
        latch.update(false, None);
        latch.update(true, Some(3));
        assert!(!latch.is_held(3));
        assert!(latch.is_held(7));
    }
}
