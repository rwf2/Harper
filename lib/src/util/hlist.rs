#[derive(Debug, Copy, Clone)]
pub struct Nil;

#[derive(Debug, Clone)]
pub struct Cons<H, T: ?Sized> {
    pub head: H,
    pub tail: T,
}

pub trait HList {
    const LEN: usize;

    fn insert<T>(self, value: T) -> Cons<T, Self>;
    fn len(&self) -> usize { Self::LEN }
    fn is_empty(&self) -> bool { Self::LEN == 0 }
}

impl HList for Nil {
    const LEN: usize = 0;

    fn insert<V>(self, value: V) -> Cons<V, Self> {
        Cons { head: value, tail: self }
    }
}

impl<H, T: HList> HList for Cons<H, T> {
    const LEN: usize = T::LEN + 1;

    fn insert<V>(self, value: V) -> Cons<V, Self> {
        Cons { head: value, tail: self }
    }
}

impl<H, T> Cons<H, T> {
    pub fn pop(self) -> (H, T) {
        (self.head, self.tail)
    }
}

pub trait Func<T> {
    type Output;

    fn call(&self, value: T) -> Self::Output;
}

pub trait Mappable<F> {
    type Output;

    fn map(self, f: F) -> Self::Output;
}

impl<F> Mappable<F> for Nil {
    type Output = Nil;

    fn map(self, _: F) -> Self::Output {
        Nil
    }
}

impl<H, F: Func<H>, T: Mappable<F>> Mappable<F> for Cons<H, T> {
    type Output = Cons<F::Output, <T as Mappable<F>>::Output>;

    fn map(self, f: F) -> Self::Output {
        Cons {
            head: F::call(&f, self.head),
            tail: self.tail.map(f)
        }
    }
}

pub trait FuncMut<T> {
    type Output;

    fn call_mut(&mut self, value: T) -> Self::Output;
}

pub trait MappableMut<F> {
    type Output;

    fn map_mut(self, f: F) -> Self::Output;
}

impl<F> MappableMut<F> for Nil {
    type Output = Nil;

    fn map_mut(self, _: F) -> Self::Output {
        Nil
    }
}

impl<H, F, T> MappableMut<F> for Cons<H, T>
    where F: FuncMut<H>,
          T: MappableMut<F>
{
    type Output = Cons<F::Output, <T as MappableMut<F>>::Output>;

    fn map_mut(self, mut f: F) -> Self::Output {
        Cons {
            head: F::call_mut(&mut f, self.head),
            tail: self.tail.map_mut(f)
        }
    }
}

pub trait Foldable<A, F> {
    type Output;

    fn fold(self, acc: A, f: F) -> Self::Output;
}

impl<A, F> Foldable<A, F> for Nil {
    type Output = A;

    fn fold(self, acc: A, _: F) -> Self::Output {
        acc
    }
}

impl<A, F, H, T> Foldable<A, F> for Cons<H, T>
    where F: Func<(H, A)>,
          T: Foldable<F::Output, F>
{
    type Output = T::Output;

    fn fold(self, acc: A, f: F) -> Self::Output {
        let output = F::call(&f, (self.head, acc));
        self.tail.fold(output, f)
    }
}

pub trait ToMut<'a> {
    type Output;

    fn to_mut(&'a mut self) -> Self::Output;
}

impl<'a> ToMut<'a> for Nil {
    type Output = Nil;

    fn to_mut(&'a mut self) -> Self::Output {
        Nil
    }
}

impl<'a, H: 'a, T: ToMut<'a>> ToMut<'a> for Cons<H, T> {
    type Output = Cons<&'a mut H, <T as ToMut<'a>>::Output>;

    fn to_mut(&'a mut self) -> Self::Output {
        Cons {
            head: &mut self.head,
            tail: self.tail.to_mut()
        }
    }
}

pub trait ToRef<'a> {
    type Output;

    fn to_ref(&'a self) -> Self::Output;
}

impl<'a> ToRef<'a> for Nil {
    type Output = Nil;

    fn to_ref(&'a self) -> Self::Output {
        Nil
    }
}

impl<'a, H: 'a, T: ToRef<'a>> ToRef<'a> for Cons<H, T> {
    type Output = Cons<&'a H, <T as ToRef<'a>>::Output>;

    fn to_ref(&'a self) -> Self::Output {
        Cons {
            head: &self.head,
            tail: self.tail.to_ref()
        }
    }
}

mod macros {
    #[doc(hidden)]
    #[macro_export]
    macro_rules! hlist {
        () => ( $crate::util::hlist::Nil );

        ($h:expr $(, $t:expr)* $(,)?) => {
            $crate::util::hlist::Cons {
                head: $h,
                tail: hlist!($($t),*)
            }
        };
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! HList {
        () => ($crate::util::hlist::Nil);

        (.. $T:ty) => ($T);

        ($H:ty) => {
            $crate::util::hlist::Cons<$H, HList![]>
        };

        ($H:ty , $($T:tt)*) => {
            $crate::util::hlist::Cons<$H, HList!($($T)*)>
        };
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! lfold {
        ([] $hlist:expr, $acc:expr, |$i:pat_param, $j:pat_param| $f:expr) => ({
            $acc
        });

        ([$dot:tt $($dots:tt)*] $hlist:expr, $acc:expr, |$i:pat_param, $j:pat_param| $f:expr) => ({
            let list = $hlist;
            let ($i, _rest) = list.pop();
            let $j = $acc;
            let acc = $f;
            $crate::util::hlist::lfold!([$($dots)*] _rest, acc, |$i, $j| $f)
        })
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! rfold {
        ([] $hlist:expr, $acc:expr, |$i:pat_param, $j:pat_param| $f:expr) => ({
            $acc
        });

        ([$dot:tt $($dots:tt)*] $hlist:expr, $acc:expr, |$i:pat_param, $j:pat_param| $f:expr) => ({
            let list = $hlist;
            let ($i, _rest) = list.pop();
            let $j = $crate::util::hlist::rfold!([$($dots)*] _rest, $acc, |$i, $j| $f);
            $f
        })
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! for_each_mut {
        ([$($dots:tt)*] $hlist:expr, |$i:pat_param| $f:expr) => (
            $crate::util::hlist::rfold!([$($dots)*] $hlist, (), |$i, _| $f)
        )
    }

    pub use {HList, hlist, rfold, lfold, for_each_mut};
}

#[doc(inline)]
pub use macros::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_map() {
        struct DebugPrint;

        impl<T: std::fmt::Debug> Func<T> for DebugPrint {
            type Output = ();

            fn call(&self, value: T) -> Self::Output {
                println!("{:?}", value);
            }
        }

        #[derive(Debug)]
        struct Foo;

        let list: HList![usize, &str, Foo] = hlist![1, "hello", Foo];
        list.map(DebugPrint);
    }

    #[test]
    fn test_generic_fold() {
        struct DebugString;

        impl<T: std::fmt::Debug> Func<(T, String)> for DebugString {
            type Output = String;

            fn call(&self, (input, acc): (T, String)) -> Self::Output {
                format!("{}, {:?}", acc, input)
            }
        }

        #[derive(Debug)]
        struct Foo;

        let list: HList![usize, &str, Foo] = hlist![1, "hello", Foo];
        list.fold("".to_string(), DebugString);
    }
}
