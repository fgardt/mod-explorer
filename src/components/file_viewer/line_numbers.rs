use leptos::prelude::*;

#[component]
pub fn LineNumbers(count: usize) -> impl IntoView {
    let numbers = (1..=count)
        .map(|i| {
            view! {
                <div>{i}</div>
            }
        })
        .collect_view();

    view! {
        <div class="line-numbers">
            {numbers}
        </div>
    }
}
