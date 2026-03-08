pub use super::simples;



pub enum AllTokensEnum {
    Simples(simples::enums::SimpleTokensEnum),
    // Agrega aquí otros enums/objetos que quieras centralizar
}

pub mod all_tokens {
    pub use super::simples::enums::SimpleTokensEnum;
    // Agrega aquí otros enums/objetos que quieras exportar centralizadamente
}

