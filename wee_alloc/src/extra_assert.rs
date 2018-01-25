#[cfg(feature = "extra_assertions")]
macro_rules! extra_assert {
    ( $condition:expr $( , $args:expr )* ) => {
        assert!($condition $( , $args )* )
    }
}

#[cfg(not(feature = "extra_assertions"))]
macro_rules! extra_assert {
    ( $condition:expr $( , $args:expr )* ) => {
        if false {
            let _ = $condition;
            $( let _ = $args; )*
        }
    }
}

#[cfg(feature = "extra_assertions")]
macro_rules! extra_assert_eq {
    ( $left:expr , $right:expr $( , $args:expr )* ) => {
        assert_eq!($left, $right $( , $args )* )
    }
}

#[cfg(not(feature = "extra_assertions"))]
macro_rules! extra_assert_eq {
    ( $left:expr , $right:expr $( , $args:expr )* ) => {
        if false {
            let _ = $left;
            let _ = $right;
            $( let _ = $args; )*
        }
    }
}

/// Define a function that only does anything when the "extra_assertions"
/// feature is enabled.
///
/// When that feature is not enabled, then the function is a no-op that is
/// marked `#![inline(always)]` and should completely disappear in the final
/// compilation artifact.
macro_rules! extra_only {
    (
        fn $name:ident $(< $($param:ident),* $(,)* >)* ( $( $arg:ident : $arg_ty:ty ),* $(,)* ) {
            $( $body:tt )*
        }
    ) => {
        #[cfg(feature = "extra_assertions")]
        fn $name $( < $( $param ),* >)* ( $($arg : $arg_ty),* ) {
            $( $body )*
        }

        #[cfg(not(feature = "extra_assertions"))]
        #[inline(always)]
        #[allow(dead_code)]
        fn $name $( < $( $param ),* >)* ( $($arg : $arg_ty),* ) {
            $( let _ = $arg; )*
        }
    }
}
