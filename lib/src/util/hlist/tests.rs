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
