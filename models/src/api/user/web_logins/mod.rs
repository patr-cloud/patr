/// The endpoint to delete a web login
mod delete_web_login;
/// The endpoint to get the details of a web login
mod get_web_login_info;
/// The endpoint to list all the web logins of a user
mod list_web_logins;

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub use self::{delete_web_login::*, get_web_login_info::*, list_web_logins::*};
use crate::prelude::*;

/// Tracks a user's web login.
///
/// Whenever a user logs into Patr via the dashboard, a new login is created.
/// This is used to track the last time a user logged in, and to allow users to
/// see all of their active sessions, and revoke them if necessary (kinda like
/// how in your Google account settings you can see which and all devices you
/// have logged into).
///
/// Most of the data will be tracked through audit logs. What is stored here
/// will be used to display things like which browser / device was used to login
/// (similar to how google shows "last login from macOS device on Safari in
/// Bangalore, India").
///
/// Now the thing is, a user's user-agent cannot change in the same login. I
/// mean, think about it: How can a user login from Safari and then the next API
/// call is from Chrome? That's not possible. If that ever happens, it usually
/// means that someone else is trying to fuck with things (like the user's JWT
/// was stolen). In these cases, we should expire the login (or at least ask
/// them to reauthenticate) and send them an email notifying them that something
/// fishy is happening, and that they should take a look at their security.
///
/// That being said, we should definitely allow for small changes in
/// user-agents. Like for example, if they update their browser version, their
/// user-agent will change to reflect the new version. We can't log them out
/// because of that.
///
/// Additionally, if a user is logging in from one city, but then the next login
/// is from a different city / country, we definitely need to ask them to
/// reauthenticate. Most hacks / attacks happen by a user stealing credentials
/// and using them to login from a different system. To prevent this, we should
/// reauthenticate the user when we detect a change in country or a major
/// difference in geo-location and send the user an email notifying them about
/// the fact that we got a suspicious login, and that they should recheck their
/// security.
///
/// What is the criteria to determine if a login needs to be reauthenticated?
/// Well, that's a tough one, to be honest. Off the top of my head, I think
/// maybe if the OS + Browser (without version) + City (or Country maybe?) +
/// Timezone combination changes, we reauthenticate. Open to suggestions on what
/// the combination should be. Basically, we calculate a checksum of the above
/// combo, and check with the checksum of the loginId that's stored in the db.
/// If it's different, we reauthenticate. We don't store the checksum, btw. We
/// calculate it every time. This is done so that in case our checksum algorithm
/// changes, we don't have stale data in the db.
///
/// Now I keep saying reauthenticate in all the above text, and to clarify, that
/// DOES NOT mean log them out. What I'm about to describe next is a very
/// complex process, but it provides an AMAZING as fuck user experience. So this
/// is not something I expect to accomplish immediately, but perhaps over time.
///
/// When something fishy is going on (like the country changes, for example), we
/// return some sort of an error code and ask the user to login again. This does
/// not mean that the current login is expired, no. Why not, you ask? Because
/// fuck you, that's why. Just kidding lol. Because it would mean that any
/// random person attempting to login from somewhere else can log a legitimate
/// user out by simply attempting a login. Doesn't make sense to inconvenience a
/// legitimate user and make things easy for a scam user.
///
/// Instead, what we do is we ask the user to reauthenticate themselves -
/// meaning their session is still stored all fine and dandy. They just need
/// to enter their password again. Not username / email, just password. This is
/// more of a "hey, just checking in to make sure that you're really who you
/// claim to be". Should they enter their MFA OTP again? Not sure. That's
/// something I still have to think about. I'm open to suggestions on that.
/// Anyways, once they reauthenticate themselves, we know that they are
/// verified. So we update the location, country, etc to the new values. This
/// way, when they send an API call again, we can check with the updated values
/// of the login and know that they're good to go. Like if you go on a trip to
/// Germany, for example, we'll detect a change in the country. Then, we ask you
/// to reauthenticate. Once you're reauthenticated, we update the DB to say your
/// login is now in Germany, and any subsequent API calls will all be fine cuz
/// the DB now says your login is in Germany. Note: Changing a browser (from
/// Safari to Chrome or macOS to Windows) is NEVER allowed. No reauthentication.
/// Straight reject and notify the user. If you want to do that, login again on
/// the new browser.
///
/// Now, you'd think we're done, but no. This shit gets wayy more complicated
/// than that. Here comes the biggest problem:
///
/// * drum roll please *
///
/// Audit log! For those living under a cave, audit logs are basically a log of
/// all actions done on the platform, used for auditing purposes (hence the
/// name, duh). Why do we care? We don't. Well, we do, mostly because we _have_
/// to for SOC2, ISO and other compliance related bullshit, but also because it
/// gives us some cool features.
///
/// Imagine this: A user logs in to Bangalore, (let's assume that login ID is 1,
/// and IP address is 1.2.3.4). They create a deployment with that login ID. Now
/// they go to Germany (idk why I keep using Germany as an example but let's
/// roll with it). Patr detects a country / login checksum change, yes? So they
/// get reauthenticated. Now let's say their new IP address is 5.6.7.8. Login ID
/// 1's IP address and country now gets updated. So far so good. Let's say the
/// deployment raked up a huge bill and the user realized their mistake and then
/// they delete the deployment. When you check the audit log to see who created
/// the deployment, it'll say it was created by our user, but in Germany from
/// the IP address 5.6.7.8 (because we updated the DB!). But that's not true. It
/// was created in Bangalore from the IP address 1.2.3.4. Well, fuck. What now?
///
/// I have one solution to this problem: when we reauthenticate, we expire the
/// old login and assign a new login to the user. Now, I know I know, this is
/// the exact opposite of what I said earlier. My previous statement still
/// holds. What I'm saying is that we make it _look_ like the user is getting
/// the same login, but internally, we can just expire the old login, never show
/// it to the user ever again, and silently switch them to a new login. This
/// way, any audit log will still retain information on what IP address the
/// request was sent from when it was sent.
///
/// Now, this does complicate things in scenarios where your IP
/// address changes, but your checksum doesn't (like if you go to a coffee shop
/// near your house). On one hand, we can't reauthenticate the user every time
/// their IP address changes (it can happen very often if you're, say, on a
/// mobile network). On the other hand, we can't afford to log wrong IP
/// addresses and stuff in our audit logs.
///
/// One middle-ground I can think of is - do a silent switching of logins if
/// _ANYTHING AT ALL_ about their login changes (IP address, checksum, user
/// agent, etc), but without a reauthentication. This does mean more data
/// logged, but unfortunately, it's something we'll need to do for these stupid
/// compliance standards. But if a big change happens that might be a potential
/// risk to the security of the user's account, then we do a reauthentication.
///
/// Again, completely open to suggestions on this. Anything that can simplify
/// this process is a win in my book.
///
/// To be clear, ALL of this is ONLY for web logins (maybe OAuth apps too, but
/// that's a whole different mess that we won't get into now). For API tokens,
/// they already have their own "allowed IP addresses" and stuff so we're good
/// there. We just need to figure out a way to store the audit logs for API
/// tokens in terms of the IP address and other stuff, if required. The problem
/// there is that the Login ID cannot silently change since it's a part of the
/// API token. So there is a single login ID for a given API token, and we need
/// to be able to audit log stuff for that. I'm not gonna break my head over
/// that now. I've documented enough already. This is now a problem for future
/// Rakshith. (PS - If I ever do figure it out and forget to update this
/// documentation, you'll find the mechanism for that somewhere in the
/// [`api-token`](super::api_token) module)
///
/// "Now hold on a minute, Rakshith", I hear you ask. "Why the heck are we even
/// bothering with all this? Why not just do a simple authentication system and
/// forget about it". Well, because security. We're not just responsible for our
/// data. We're responsible for our user's data AND our user's user's data.
/// Companies spend billions of dollars on security, and we're building a
/// platform that will be primarily used by companies, and we need to make
/// sure that we're doing everything we can to protect their data. Additionally,
/// if our user's Patr account gets hacked, ALL their data AS WELL AS their
/// user's data gets leaked. That's a huge problem. So we need to make sure that
/// we not only secure things for our users, but also inform them about security
/// events that might affect their account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserWebLogin {
	/// The time at which this login expires. If the expiry has elapsed the user
	/// should be automatically logged out.
	pub token_expiry: OffsetDateTime,
	/// When this login was created.
	pub created: OffsetDateTime,
	/// Which IP address this login was created from
	pub created_ip: IpAddr,
	/// The geo-location of the IP address from which this login was created.
	/// We'll probably need something like ipinfo.io for getting this
	/// information.
	pub created_location: GeoLocation,
	/// The user-agent of the browser that was used to create this login. We'll
	/// need some strict CORS settings to make sure that people can't call our
	/// login API from outside a browser, so that we get actual valid
	/// user-agents here. Also, unknown user-agents can maybe be rejected? Or
	/// would that cause too many false positives? Need to test and find out.
	pub created_user_agent: String,
	/// The country from which this login was created. This is again derived
	/// from an IP service like ipinfo.io
	pub created_country: String,
	/// The region from which this login was created. ipinfo.io to the rescue.
	pub created_region: String,
	/// The city from which this login was created. ipinfo.io again.
	pub created_city: String,
	/// The timezone of the IP address from which this login was created. This
	/// is calculated using.....yeah no, ipinfo.io again.
	pub created_timezone: String,
}

#[cfg(test)]
mod test {
	use std::net::{IpAddr, Ipv4Addr};

	use serde_test::{assert_tokens, Configure, Token};
	use time::OffsetDateTime;

	use super::UserWebLogin;
	use crate::prelude::*;

	#[test]
	fn assert_user_login_types() {
		assert_tokens(
			&UserWebLogin {
				token_expiry: OffsetDateTime::UNIX_EPOCH,
				created: OffsetDateTime::UNIX_EPOCH,
				created_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
				created_location: GeoLocation {
					latitude: 0.0,
					longitude: 0.0,
				},
				created_user_agent: "user-agent".to_string(),
				created_country: "IN".to_string(),
				created_region: "Karnataka".to_string(),
				created_city: "Bengaluru".to_string(),
				created_timezone: "UTC".to_string(),
			}
			.readable(),
			&[
				Token::Struct {
					name: "UserWebLogin",
					len: 9,
				},
				Token::Str("tokenExpiry"),
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::Str("created"),
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::Str("createdIp"),
				Token::Str("127.0.0.1"),
				Token::Str("createdLocation"),
				Token::Struct {
					name: "GeoLocation",
					len: 2,
				},
				Token::Str("latitude"),
				Token::F64(0.0),
				Token::Str("longitude"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("createdUserAgent"),
				Token::Str("user-agent"),
				Token::Str("createdCountry"),
				Token::Str("IN"),
				Token::Str("createdRegion"),
				Token::Str("Karnataka"),
				Token::Str("createdCity"),
				Token::Str("Bengaluru"),
				Token::Str("createdTimezone"),
				Token::Str("UTC"),
				Token::StructEnd,
			],
		);
	}
}
