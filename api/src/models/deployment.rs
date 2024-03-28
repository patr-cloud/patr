use std::collections::HashMap;
use models::utils::Uuid;
use once_cell::sync::OnceCell;

pub static MACHINE_TYPES: OnceCell<HashMap<Uuid, (i16, i32)>> = OnceCell::new();