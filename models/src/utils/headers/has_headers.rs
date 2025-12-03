use headers::{authorization::Credentials, *};

use super::Header;

/// This struct is implemented for all types that can be used as a header in a
/// request to the API.
///
/// This struct is used in conjunction with the [`HasHeaders`] trait to ensure
/// that a request headers struct has all the required headers that are needed
/// for the query, body, etc.
///
/// This should be implemented for any struct that defines a header. It is
/// already implemented for all types that implement the [`Header`] trait
/// in the [`headers`] crate.
pub trait HasHeader<H>
where
	H: Header,
{
	/// A helper function that returns a reference to the header. Not really
	/// used much in the codebase yet, but kept just for safe measure.
	fn get_header(&self) -> &H;
}

impl<H> HasHeader<H> for H
where
	H: Header,
{
	fn get_header(&self) -> &H {
		self
	}
}

/// This trait is implemented with tuples of elements as a generic (up to 16
/// elements) for any struct that has those headers.
///
/// It is used to ensure that a request headers struct has all the required
/// headers that are needed for the query, body, etc.
///
/// For example, given a struct `Foo` that has the headers `A` and `B`,
/// `HasHeaders<(A, B)>` is automatically implemented for `Foo` IF AND ONLY IF
/// `Foo` implements `HasHeader<A>` and `HasHeader<B>`.
///
/// More realistic example:
///
/// Given a struct `RequestHeaders` like so:
/// ```rust
/// # use headers::{AcceptRanges, ContentType};
/// pub struct RequestHeaders {
///     pub accept: AcceptRanges,
///     pub content_type: ContentType,
/// }
/// ```
///
/// If `RequestHeaders` implements `HasHeader<AcceptRanges>` and
/// `HasHeader<ContentType>` like so:
///
/// ```rust
/// # use headers::{AcceptRanges, ContentType};
/// # pub struct RequestHeaders {
/// #    pub accept: AcceptRanges,
/// #    pub content_type: ContentType,
/// # }
/// # use models::utils::{HasHeaders, HasHeader};
/// impl HasHeader<AcceptRanges> for RequestHeaders {
///     fn get_header(&self) -> &AcceptRanges {
///         &self.accept
///     }
/// }
///
/// impl HasHeader<ContentType> for RequestHeaders {
///     fn get_header(&self) -> &ContentType {
///         &self.content_type
///     }
/// }
/// ```
///
/// Then `HasHeaders<(AcceptRanges, ContentType)>` is automatically implemented
/// for `RequestHeaders`.
///
/// Now, it is indeed cumbersome to implement [`HasHeader`] for every header in
/// a struct. So, the [`macros::HasHeaders`] derive macro can be used to
/// automatically implement `HasHeader` for all headers in a given struct.
///
/// In the above example, the following code is equivalent to the code above:
/// ```rust
/// # use headers::{AcceptRanges, ContentType};
/// # use macros::HasHeaders;
/// # use models::utils::HasHeaders;
/// #[derive(HasHeaders)]
/// pub struct RequestHeaders {
///     pub accept: AcceptRanges,
///     pub content_type: ContentType,
/// }
/// ```
///
/// Now, we can make a function that requires a `RequestHeader` to have these
/// two headers necessarily by using the [`HasHeaders`] trait:
/// ```rust
/// # use headers::{AcceptRanges, ContentType};
/// # use models::utils::{HasHeaders, HasHeader};
/// // A function that requires the `AcceptRanges` and `Content-Type` headers
/// fn foo<T>(headers: &T)
/// where
///     T: HasHeaders<(AcceptRanges, ContentType)>,
/// #    T: HasHeader<AcceptRanges>,
/// #    T: HasHeader<ContentType>,
/// {
///     // ...
///     let accept: &AcceptRanges = headers.get_header();
///     let content_type: &ContentType = headers.get_header();
/// }
/// ```
///
/// The best part is that in the above example, even if `T` has more headers, it
/// will still work. This way, we can make functions that require a certain set
/// of headers, and the user can pass in any struct that has those headers, even
/// if it has more headers.
pub trait HasHeaders<T> {}

/// This macro is used to implement [`HasHeaders`] for a struct. It is used to
/// automatically implement [`HasHeader`] for all headers in a given struct.
///
/// If a struct implements [`HasHeader`] for all headers in the tuple, then
/// [`HasHeaders`] is automatically implemented for that struct.
///
/// If you want to accept a struct that has certain headers, you can use the
/// [`HasHeaders`] trait to do so. For example:
/// ```rust
/// # use models::utils::{HasHeaders, HasHeader};
/// # use headers::AcceptRanges;
/// #
/// pub struct RequestHeaders {
///     pub accept: AcceptRanges,
/// }
///
/// impl HasHeader<AcceptRanges> for RequestHeaders {
///     fn get_header(&self) -> &AcceptRanges {
///         &self.accept
///     }
/// }
/// ```
///
/// ```rust
/// # use models::utils::{HasHeaders, HasHeader};
/// # use headers::AcceptRanges;
/// # use macros::HasHeaders;
/// #
/// // This is equivalent to the above code
/// #[derive(HasHeaders)]
/// pub struct RequestHeaders {
///     pub accept: AcceptRanges,
/// }
///
/// // A function that requires the `AcceptRanges` header
/// fn foo<T>(headers: &T)
/// where
///     T: HasHeaders<(AcceptRanges,)>,
/// #    T: HasHeader<AcceptRanges>,
/// {
///     // ...
///     let accept: &AcceptRanges = headers.get_header();
/// }
/// ```
///
/// For more details, see the documentation for [`HasHeaders`].
macro_rules! impl_has_headers {
	() => {
		impl<S> HasHeaders<()> for S {}
	};
	( $($headers:ident),+ $(,)? ) => {
		impl<$($headers,)* S> HasHeaders<($($headers,)*)> for S
		where
			$($headers: Header,)*
			S: $(HasHeader<$headers> +)*
		{
		}
	};
}

impl_has_headers!();
impl_has_headers!(H1);
impl_has_headers!(H1, H2);
impl_has_headers!(H1, H2, H3);
impl_has_headers!(H1, H2, H3, H4);
impl_has_headers!(H1, H2, H3, H4, H5);
impl_has_headers!(H1, H2, H3, H4, H5, H6);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14);
impl_has_headers!(
	H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15
);
impl_has_headers!(
	H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15, H16
);

/// This trait is implemented for all types that can be used as a header in a
/// request to the API. This struct is used in conjunction with the
/// [`HasHeaders`] trait to ensure that a request headers struct has all the
/// required headers that are needed for the query, body, etc.
macro_rules! impl_has_headers_for_standard_header {
	[$($header:ident),+ $(,)?] => {
		$(impl HasHeaders<$header> for $header {})+
	};
}

impl_has_headers_for_standard_header![
	AcceptRanges,
	AccessControlAllowCredentials,
	AccessControlAllowHeaders,
	AccessControlAllowMethods,
	AccessControlAllowOrigin,
	AccessControlExposeHeaders,
	AccessControlMaxAge,
	AccessControlRequestHeaders,
	AccessControlRequestMethod,
	Age,
	Allow,
	CacheControl,
	Connection,
	ContentDisposition,
	ContentEncoding,
	ContentLength,
	ContentLocation,
	ContentRange,
	ContentType,
	Cookie,
	Date,
	ETag,
	Expect,
	Expires,
	Host,
	IfMatch,
	IfModifiedSince,
	IfNoneMatch,
	IfRange,
	IfUnmodifiedSince,
	LastModified,
	Location,
	Origin,
	Pragma,
	Range,
	Referer,
	ReferrerPolicy,
	RetryAfter,
	SecWebsocketAccept,
	SecWebsocketKey,
	SecWebsocketVersion,
	Server,
	SetCookie,
	StrictTransportSecurity,
	Te,
	TransferEncoding,
	Upgrade,
	UserAgent,
	Vary,
];

impl<C> HasHeaders<Authorization<C>> for Authorization<C> where C: Credentials {}
impl<C> HasHeaders<ProxyAuthorization<C>> for ProxyAuthorization<C> where C: Credentials {}
