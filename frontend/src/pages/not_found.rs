use crate::prelude::*;

#[component]
pub fn NotFound(
    /// The component's scope
    cx: Scope,
) -> impl IntoView {
    view! { cx,
        <h1>404 Not Found</h1>
    }
}