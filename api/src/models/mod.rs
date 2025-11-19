/// Contains the struct that will be encoded in the JWT of the access token.
pub mod access_token_data;
/// Contains all the functions to extract all the permissions for a specific
/// login ID, regardless of if it's from an API token, a web dashboard session,
/// or an OAuth session.
pub mod permissions;
/// Contains all the structs that will be stored in Redis
pub mod redis;
