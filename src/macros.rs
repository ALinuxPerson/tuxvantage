macro_rules! log {
    (
        rust devs: allow $d:tt to be used in nested macro rules repititions please! anyways;
        $($fn_name:ident,)*
    ) => {
        $(
        #[macro_export]
        macro_rules! $fn_name {
            (@prologue $d ($d arg:tt)*) => {
                 $crate::log::$fn_name(::std::format_args!($d ($d arg)*), true)
            };
            (@no-prologue $d ($d arg:tt)*) => {
                 $crate::log::$fn_name(::std::format_args!($d ($d arg)*), false)
            };
            (@np $d ($d arg:tt)*) => {
                $fn_name!(@no-prologue $d ($d arg)*)
            };
            (@p $d ($d arg:tt)*) => {
                $fn_name!(@prologue $d ($d arg)*)
            };
            ($d ($d arg:tt)*) => {
                $fn_name!(@p $d ($d arg)*)
            };
        }
        )*
    };
}

log! {
    rust devs: allow $ to be used in nested macro rules repititions please! anyways;
    error,
    warn,
    info,
    tip,
    debug,
}
