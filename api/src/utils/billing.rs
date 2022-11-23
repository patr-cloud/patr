pub fn cents_to_dollars(cents: u64) -> String {
	format!("{:.2}", cents as f64 * 0.01)
}

pub fn stringify_month(month: u8) -> &'static str {
	match month {
		1 => "January",
		2 => "February",
		3 => "March",
		4 => "April",
		5 => "May",
		6 => "June",
		7 => "July",
		8 => "August",
		9 => "September",
		10 => "October",
		11 => "November",
		12 => "December",
		_ => "Invalid month",
	}
}
