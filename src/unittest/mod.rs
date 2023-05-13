pub mod setup {
    pub mod log {
        use std::sync::Once;
        static SETUP: Once = Once::new();
        pub fn configure() {
            SETUP.call_once(|| {
                let _ = env_logger::builder()
                    // .is_test(true) // disables color in the terminal
                    .filter_level(log::LevelFilter::Trace)
                    .try_init();
            });
        }
    }
}
