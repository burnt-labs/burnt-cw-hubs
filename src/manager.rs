pub mod contract_manager {
    use cosmwasm_std::StdError;
    use cw_storage_plus::Item;
    use std::{cell::RefCell, rc::Rc};
    use thiserror::Error;

    use burnt_glue::manager::Manager;
    use metadata::Metadata;
    use ownable::Ownable;

    use crate::state::HubMetadata;

    #[derive(Error, Debug)]
    pub enum ManagerError {
        #[error("{0}")]
        Std(#[from] StdError),

        #[error("Custom Error val: {val:?}")]
        CustomError { val: String },
        // Add any other custom errors you like here.
        // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    }
    
    pub fn get_manager() -> Manager {
        let mut contract_manager = Manager::new();
        // register all modules required for call
        let owner: Rc<RefCell<Ownable>> = Rc::new(RefCell::new(Ownable::default()));
        contract_manager
            .register("ownable".to_string(), owner.clone())
            .unwrap();

        let metadata =
            Rc::new(RefCell::new(Metadata::new(Item::<HubMetadata>::new("metadata"), owner.clone())));
        contract_manager
            .register("metadata".to_string(), metadata)
            .unwrap();
        contract_manager
    }
}
