#[macro_export]
macro_rules! oprintln {
    ($out:ident, $($arg:tt)*) => {
        $out.push(format!($($arg)*));
        println!($($arg)*);
    };
}

mod details;
pub use details::details;

mod verse;
pub use verse::verse;
