
/// converts a number to a string with thousands separator
#[macro_export]
macro_rules! fmt_num {
    ($num:expr) => {
        num_format::ToFormattedString::to_formatted_string(&$num, &num_format::Locale::en)
    };
}
