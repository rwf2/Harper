pub unsafe trait Variation {
    type Original;
}

#[macro_export]
macro_rules! declare_variation {
    ($v:vis $V:ident of $T:ty) => {
        #[repr(transparent)]
        $v struct $V(pub $T);

        impl std::ops::Deref for $V {
            type Target = $T;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        unsafe impl $crate::util::Variation for $V {
            type Original = $T;
        }

        #[allow(unused)]
        impl $V {
            #[inline(always)]
            $v fn new(other: std::sync::Arc<$T>) -> std::sync::Arc<$V> {
                unsafe {
                    let inner = std::sync::Arc::into_raw(other) as *const $V;
                    std::sync::Arc::from_raw(inner)
                }
            }

            $v fn from<V>(other: &std::sync::Arc<V>) -> std::sync::Arc<$V>
                where V: $crate::util::Variation<Original = <Self as $crate::util::Variation>::Original>
            {
                unsafe {
                    let inner = std::sync::Arc::into_raw(other.clone()) as *const $V;
                    std::sync::Arc::from_raw(inner)
                }
            }

            $v fn as_original<'a>(self: &'a std::sync::Arc<Self>) -> &'a std::sync::Arc<$T> {
                unsafe {
                    std::mem::transmute(self)
                }
            }
        }
    };
}

pub use declare_variation;
