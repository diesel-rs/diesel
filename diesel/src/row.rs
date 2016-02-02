use backend::Backend;

pub trait Row<DB: Backend> {
    fn take(&mut self) -> Option<&DB::RawValue>;
    fn next_is_null(&self, count: usize) -> bool;
}
