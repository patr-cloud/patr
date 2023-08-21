macros::declare_api_endpoint!(
	IsCouponValid,
	GET "/auth/coupon-valid",
	query = {
		pub coupon: String,
	},
	response = {
		pub valid: bool,
	}
);
