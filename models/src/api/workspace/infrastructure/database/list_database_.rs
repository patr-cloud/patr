use crate::prelude::*;

macros::declare_api_endpoint!(
	// Get list of all database
	ListDatabase,
	GET "/workspace/:workspace_id/infrastructure/database",
    path = {
        pub workspace_id: Uuid
    },
	response = {
		pub database: Vec<Database>
	}
);
