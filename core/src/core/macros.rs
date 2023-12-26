/// converts a number to a string with thousands separator
#[cfg(feature = "unittest")]
#[macro_export]
macro_rules! fmt_num {
    ($num:expr) => {
        num_format::ToFormattedString::to_formatted_string(&$num, &num_format::Locale::en)
    };
}

pub fn short_type_name<T: ?Sized>() -> &'static str {
    use std::any::type_name;
    type_name::<T>().split('<').next().unwrap().split("::").last().unwrap_or("Unknown")
}

pub fn short_instance_type_name<T: Sized>(_: T) -> &'static str {
    use std::any::type_name;
    type_name::<T>().split('<').next().unwrap().split("::").last().unwrap_or("Unknown")
}

#[cfg(debug_assertions)]
#[track_caller]
pub fn ty_name<T: ?Sized>(name: &'static str) -> &'static str {
    use std::any::type_name;
    let expected_short_name = type_name::<T>().split('<').next().unwrap().split("::").last().unwrap_or("Unknown");
    debug_assert_eq!(name, expected_short_name, "Please check that you correct manual Debug & Display impl after refactoring");
    expected_short_name
}
#[cfg(not(debug_assertions))]
pub fn ty_name<T: ?Sized>(name: &'static str) -> &'static str {
    name
}
/// Will endure that the short name of Self is matching the name of struct, resolved via [Self] argument.
/// This prevents invalid names in Debug/Display output after refactoring without performance penalty at run time.
/// Will panic in debug build only if the name is not matching and always resolves to the literal name in release build.
#[macro_export]
macro_rules! asserted_short_name {
    ($name:literal, $ty:ty) => {
        $crate::core::macros::ty_name::<$ty>($name)
    };
}
