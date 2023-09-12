use crate::prelude::*;

macros::declare_api_endpoint!(
	// To check if coupon is valid
	IsCouponValid,
	GET "/auth/coupon-valid",
	request = {
		pub coupon: String,
	},
	response = {
		pub valid: bool,
	}
);
