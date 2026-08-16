
/// Funciones host que el runtime importa del entorno
pub struct HostApi {
    pub print_fn: Option<Box<dyn Fn(&str)>>,
    pub input_fn: Option<Box<dyn Fn() -> String>>,
}

impl HostApi {
    pub fn new() -> Self {
        Self {
            print_fn: None,
            input_fn: None,
        }
    }
}
