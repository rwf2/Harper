use std::{sync::Arc, ops::Deref};
use once_cell::sync::Lazy;

type LazyResult<T, E> = Lazy<Result<T, E>, Box<dyn FnOnce() -> Result<T, E> + Send + Sync>>;

#[derive(Debug)]
pub struct LazyFallibleArc<T, E>(Arc<LazyResult<T, E>>);

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> LazyFallibleArc<T, E> {
    #[inline(always)]
    pub fn force_in_background(&self) {
        let lazy = self.0.clone();
        rayon::spawn(move || { Lazy::force(&lazy); });
    }

    pub fn force(&self) -> Result<&T, &E> {
        Lazy::force(&*self.0).as_ref()
    }
}

impl<T, E> Clone for LazyFallibleArc<T, E> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, E> Deref for LazyFallibleArc<T, E> {
    type Target = Result<T, E>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}

impl<T, E> LazyFallibleArc<T, E> {
    #[inline(always)]
    pub fn new<F>(with: F) -> Self
        where F: FnOnce() -> Result<T, E> + Send + Sync + 'static
    {
        LazyFallibleArc(Arc::new(Lazy::new(Box::new(with))))
    }
}
