use std;

pub trait Omittable {
    fn omit(&self) {}
    fn o(&self) {}
}

impl<T, E> Omittable for std::result::Result<T, E> {}
impl Omittable for () {}
