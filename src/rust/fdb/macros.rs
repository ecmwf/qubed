#[macro_export]
macro_rules! generate_library_wrapper {
    (
        $lib_name:ident {
            $(
                fn $func_name:ident($($arg_name:ident : $arg_type:ty),* $(,)?) $(-> $ret_type:ty)?;
            )*
        }
    ) => {

        pub struct $lib_name {
            lib: Arc<libloading::Library>,
            $(
                pub $func_name: unsafe extern "C" fn($($arg_type),*) $(-> $ret_type)?,
            )*
        }

        impl $lib_name {
            pub fn load(lib: libloading::Library) -> Result<Self, Box<dyn std::error::Error>> {
                let arc_lib = Arc::new(lib);
                Ok(Self {
                    $(
                        $func_name: unsafe {
                            *arc_lib.get::<unsafe extern "C" fn($($arg_type),*) $(-> $ret_type)?>(concat!(stringify!($func_name), "\0").as_bytes())?
                        },
                    )*
                    lib: arc_lib,
                })
            }
        }
    };
}
