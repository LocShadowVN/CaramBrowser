use leptos::*;

#[component]
pub fn NewTabView<F>(on_navigate: F) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{
    view! {
        <div class="internal-view">
            <div class="newtab-container">
                <div class="newtab-clock">"12:00"</div>
                <p style="color:var(--text-secondary); margin-bottom:20px;">"Protected by Caram Shield Core"</p>

                <div class="privacy-dash">
                    <div class="dash-box">
                        <div class="val">"14,290"</div>
                        <div class="lbl">"Trackers & Ads Blocked"</div>
                    </div>
                    <div class="dash-box">
                        <div class="val">"178 MB"</div>
                        <div class="lbl">"Bandwidth Saved"</div>
                    </div>
                    <div class="dash-box">
                        <div class="val">"14.3 s"</div>
                        <div class="lbl">"Time Saved"</div>
                    </div>
                </div>

                <div class="speed-dial-grid">
                    <div class="dial-item" on:click=move |_| on_navigate("https://duckduckgo.com".into())>
                        <strong>"DuckDuckGo"</strong>
                        <span style="font-size:11px; color:var(--text-secondary)">"Search Engine"</span>
                    </div>
                    <div class="dial-item" on:click=move |_| on_navigate("https://github.com".into())>
                        <strong>"GitHub"</strong>
                        <span style="font-size:11px; color:var(--text-secondary)">"Code Hosting"</span>
                    </div>
                    <div class="dial-item" on:click=move |_| on_navigate("https://rust-lang.org".into())>
                        <strong>"Rust Lang"</strong>
                        <span style="font-size:11px; color:var(--text-secondary)">"Systems Language"</span>
                    </div>
                    <div class="dial-item" on:click=move |_| on_navigate("https://cloudflare.com".into())>
                        <strong>"Cloudflare"</strong>
                        <span style="font-size:11px; color:var(--text-secondary)">"Edge Network"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}
