use crate::prelude::*;

macros::declare_api_endpoint!(
	// Definition of a route to validate if a coupon is legitimate or not
	IsCouponValid,
	GET "/auth/coupon-valid",
	query = {
		// The coupon to be validated
		pub coupon: String,
	},
	response = {
		// A boolean response corresponding the availability of the coupon
		pub valid: bool,
	}
);
