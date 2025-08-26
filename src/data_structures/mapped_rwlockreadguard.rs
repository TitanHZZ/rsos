use spin::RwLockReadGuard;
use core::ops::Deref;

pub struct MappedRwLockReadGuard<'a, T: 'a + ?Sized, U: 'a + ?Sized> {
    guard: RwLockReadGuard<'a, T>,
    mapper: fn(&T) -> &U,
}

impl<'a, T: 'a + ?Sized, U: 'a + ?Sized> MappedRwLockReadGuard<'a, T, U> {
    pub fn new(guard: RwLockReadGuard<'a, T>, mapper: fn(&T) -> &U) -> Self {
        MappedRwLockReadGuard { guard, mapper }
    }
}

impl<'a, T: 'a + ?Sized, U: 'a + ?Sized> Deref for MappedRwLockReadGuard<'a, T, U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        (self.mapper)(&self.guard)
    }
}
