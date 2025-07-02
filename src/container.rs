use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MyVec<T>(Vec<T>);

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
    pub(crate) fn new() -> Self {
        MyVec(vec![])
    }
    /*
        pub(crate) fn push(&mut self, value: T) {
            self.0.push(value);
        }
        pub(crate) fn pop(&mut self) -> Option<T> {
            self.0.pop()
        }
        pub(crate) fn split_off(&mut self, at: usize) -> MyVec<T> {
            let tail = self.0.split_off(at);
            MyVec(tail)
        }
        pub(crate) fn len(&self) -> usize {
            self.0.len()
        }
        pub(crate) fn is_empty(&self) -> bool {
            self.len() == 0
        }
    */
}

impl<T> FromIterator<T> for MyVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/*
impl<T> Default for MyVec<T> {
    fn default() -> Self {
        Self(vec![])
    }
}


impl<T> Deref for MyVec<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> IntoIterator for MyVec<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a MyVec<T> {
    type Item = &'a T;
    type IntoIter = <&'a Vec<T> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
*/
