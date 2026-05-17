use leptos::prelude::*;

const CODE: &str = r#"
if (window.location.host == "mods.factorio.com") {
    window.location.href = "[TARGET]" + window.location.pathname.match(/\/(?:mod|user)\/[^/?#]+/)[0]
}
"#
.trim_ascii();

#[component]
pub fn Bookmarklet() -> impl IntoView {
    // Broken: https://github.com/leptos-rs/leptos/issues/4153
    // let own_url = use_url().read_untracked();
    // let origin = own_url.origin();

    let code = CODE
        .replace(['\n', ' '], "")
        .replace("[TARGET]", "https://mod.tools.bpbin.com");
    let code = urlencoding::encode(&code);
    let code = format!("javascript:(function(){{{code}}})()");

    view! {
        <div class="bookmarklet">
            <h3>"For an even simpler way to inspect a mod from the portal you can use this bookmarklet"</h3>
            <a href=code>"inspect mod"</a>
            <p>"Just drag it into your bookmarks and click it while on the mod portal"</p>
        </div>
    }
}
