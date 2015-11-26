use expression::count::*;
use super::SelectDsl;

pub trait CountDsl: SelectDsl<CountStar> + Sized {
    fn count(self) -> <Self as SelectDsl<CountStar>>::Output {
        self.select(count_star())
    }
}

impl<T: SelectDsl<CountStar>> CountDsl for T {}
