#[doc(hidden)]
#[macro_export]
macro_rules! hmap {
    ($($key:expr => $value:expr),* $(,)?) => ({
        #[allow(unused_mut)]
        let mut map = std::collections::HashMap::new();
        $(map.insert($key, $value);)*
        map
    });
}

#[doc(hidden)]
#[macro_export]
macro_rules! dict {
    ($($key:expr => $value:expr),* $(,)?) => ({
        let mut dict = $crate::value::Dict::new();
        $(dict.insert($key.into(), $value.into());)*
        dict
    });
}

#[doc(hidden)]
#[macro_export]
macro_rules! time {
    ($($token:tt)*) => ({
        let start = std::time::Instant::now();
        let value = { $($token)* };
        println!("{} {}:{} took {}ms",
            stringify!($($token)*), file!(), line!(), start.elapsed().as_millis());

        value
    });
}

pub use {hmap, dict, time};
