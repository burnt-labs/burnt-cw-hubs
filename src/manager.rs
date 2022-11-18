pub mod manager {
    use std::{rc::Rc, cell::RefCell};
    use cosmwasm_std::StdError;
    use thiserror::Error;

    use burnt_glue::{manager::Manager};
    use ownable::{Ownable};
    use serde_json::{Value, Value::Object};

    #[derive(Error, Debug)]
    pub enum ManagerError {
        #[error("{0}")]
        Std(#[from] StdError),

        #[error("Custom Error val: {val:?}")]
        CustomError { val: String },
        // Add any other custom errors you like here.
        // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    }

    pub fn get_manager(modules: String) -> Manager {
        let mut contract_manager = Manager::new();
        // register all modules required for call
        let vals: Value = serde_json::from_str(modules.as_str()).unwrap();
        if let Object(obj) = vals {
            let modules: Vec<(String, Value)> = obj.into_iter().collect();
            match &modules[..] {
                [(module_name, _module)] => {
                    match module_name.as_str() {
                        "ownable" => {
                            let owner: Rc<RefCell<Ownable>> = Rc::new(RefCell::new(Ownable::default()));
                            contract_manager.register("ownable".to_string(), owner).unwrap();
                        }
                        _ => ()
                    }
                }
                _ => (),
            }
        }
        return contract_manager;
    }
}