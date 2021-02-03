use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac,
	pin_fn,
	utils::{
		constants::request_keys,
		EveContext,
		EveMiddleware,
	},
};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use eve_rs::{App as EveApp, Context, Error, NextHandler};

/// addUser/
/// getBashScript/
/// deleteTunnel/

// todo: implement token auth.
pub fn creare_sub_app(app : &App) {
    let sub_app = create_eve_app(app);

    sub_app.post("/addUser",
    &[
        EveMiddleware::CustomFunction(pin_fn!(add_user))
    ]
    )

}

/// function to create new user in linux machine
async fn add_user(mut context : EveContext,  _: NextHandler<EveContext>) 
    -> Result<EveContext, Error<EveContext>> {

        let body = context.get_body_object().clone();
        // get user name from 
        let username = &context.get_token_data().unwrap().user;

        // generate unique password
        let generated_password = generate_password(10);

        Ok(context)
}


// util function 
/// generates random password for the given user.
pub fn generate_password(length : u16) -> String {
	let password: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length.into())
        .map(char::from)
		.collect();
		
	return password;
}