use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MyVec<T>(Vec<T>);

impl<T> Deref for MyVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        MyVec(vec![])
    }
}

impl<T> FromIterator<T> for MyVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}
