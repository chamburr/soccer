use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use heapless::{FnvIndexMap, String, Vec};
use serde::Deserialize;

type Variable = String<16>;
type VariableMap = FnvIndexMap<&'static str, Variable, 32>;
type Function = Vec<&'static str, 4>;
type FunctionMap = FnvIndexMap<&'static str, Function, 16>;

pub type FunctionArgs = FnvIndexMap<String<32>, String<32>, 4>;

pub static VARIABLES: Mutex<CriticalSectionRawMutex, VariableMap> = Mutex::new(FnvIndexMap::new());
pub static FUNCTIONS: Mutex<CriticalSectionRawMutex, FunctionMap> = Mutex::new(FnvIndexMap::new());

#[derive(Deserialize)]
pub struct FunctionCall {
    pub name: String<32>,
    pub args: String<256>,
}

macro_rules! debug_variable {
    ($name:literal, $value:expr) => {{
        let value = heapless::format!("{}", $value).unwrap_or("Unknown".try_into().unwrap());
        let mut variables = crate::utils::debug::VARIABLES.lock().await;
        let _ = variables.insert($name, value);
        info!("{}: {}", $name, $value);
    }};
}

macro_rules! debug_functions {
    ($(async fn $name:ident($($arg:ident: $type:ty), *) $func:expr)*) => {
        $(
            #[allow(unused_mut, unused_variables)]
            async fn $name(mut args: crate::utils::debug::FunctionArgs) {
                $(
                    let $arg: $type = args
                        .remove(&stringify!($arg).try_into().unwrap())
                        .unwrap()
                        .as_str()
                        .parse()
                        .unwrap();
                )*
                $func
            }
        )*

        pub async fn init() {
            $(
                #[allow(unused_mut)]
                let mut arguments = heapless::Vec::new();
                $(
                    let _ = arguments.push(
                        stringify!($arg)
                            .try_into()
                            .unwrap_or("Unknown".try_into().unwrap()),
                    );
                )*
                let _ = (*(crate::utils::debug::FUNCTIONS.lock().await))
                    .insert(stringify!($name), arguments);
            )*
        }

        pub async fn call_function(call: crate::utils::debug::FunctionCall) {
            let mut args: crate::utils::debug::FunctionArgs = heapless::FnvIndexMap::new();

            if call.args != "x" {
                for arg in call.args.split(',') {
                    let (key, value) = arg.split_once('=').unwrap();
                    let _ = args.insert(key.try_into().unwrap(), value.try_into().unwrap());
                }
            }

            match call.name.as_str() {
                $(
                    stringify!($name) => $name(args).await,
                )*
                _ => ()
            }
        }
    };
}

pub(crate) use debug_functions;
pub(crate) use debug_variable;

pub async fn get_variables() -> VariableMap {
    let variables = VARIABLES.lock().await;
    variables.clone()
}

pub async fn get_functions() -> FunctionMap {
    let functions = FUNCTIONS.lock().await;
    functions.clone()
}
