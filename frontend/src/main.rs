// Thay thế khối <div class="nav-bar"> trong frontend/src/main.rs:
view! {
    <div class="nav-bar">
        <button
            class="icon-btn"
            title="Go Back"
            on:click=move |_| {
                let cur = active_tab_id.get();
                let list = tabs.get();
                if let Some(tab) = list.iter().find(|t| t.id == cur) {
                    if tab.page_mode == PageMode::Web {
                        spawn_local(async move {
                            let _ = call_tauri::<_, ()>("webview_go_back", &EmptyArgs {}).await;
                        });
                    }
                }
            }
        ><IconBack /></button>

        <button
            class="icon-btn"
            title="Go Forward"
            on:click=move |_| {
                let cur = active_tab_id.get();
                let list = tabs.get();
                if let Some(tab) = list.iter().find(|t| t.id == cur) {
                    if tab.page_mode == PageMode::Web {
                        spawn_local(async move {
                            let _ = call_tauri::<_, ()>("webview_go_forward", &EmptyArgs {}).await;
                        });
                    }
                }
            }
        ><IconForward /></button>

        <button
            class="icon-btn"
            title="Reload Page"
            on:click=move |_| {
                let cur = active_tab_id.get();
                let list = tabs.get();
                if let Some(tab) = list.iter().find(|t| t.id == cur) {
                    if tab.page_mode == PageMode::Web {
                        spawn_local(async move {
                            let _ = call_tauri::<_, ()>("webview_reload", &EmptyArgs {}).await;
                        });
                    } else {
                        navigate(tab.url.clone(), false);
                    }
                }
            }
        ><IconReload /></button>

        <div class="omnibox-box">
            <div class="lock-icon"><IconLock /></div>
            <input
                type="text"
                class="omnibox-input"
                placeholder="Search web or enter address (Auto-Cleaned & De-AMP)"
                prop:value=omnibox_text
                on:input=move |ev| set_omnibox_text.set(event_target_value(&ev))
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        navigate(omnibox_text.get(), true);
                    }
                }
            />

            {move || {
                let creds = available_credentials.get();
                if !creds.is_empty() {
                    let first = creds[0].clone();
                    view! {
                        <button
                            class="autofill-btn"
                            title=format!("Autofill as {}", first.username)
                            on:click=move |_| {
                                let u = first.username.clone();
                                let s = first.secret.clone();
                                spawn_local(async move {
                                    let _ = call_tauri::<_, ()>("execute_autofill", &ExecuteAutofillArgs {
                                        username: u,
                                        secret: s,
                                    }).await;
                                });
                            }
                        >
                            <IconKey />
                            <span>"Autofill"</span>
                        </button>
                    }.into_view()
                } else {
                    view! { <div style="display:none;"></div> }.into_view()
                }
            }}

            <button class="shield-btn" on:click=move |_| set_shield_open.set(!shield_open.get())>
                <IconShield />
                <span>{move || {
                    let cur_id = active_tab_id.get();
                    tabs.get().into_iter().find(|t| t.id == cur_id).map(|x| x.blocked_count).unwrap_or(0)
                }}</span>
            </button>

            <button class="icon-btn" on:click=move |_| {
                let cur_url = omnibox_text.get();
                if !cur_url.is_empty() && !cur_url.starts_with("caram://") {
                    spawn_local(async move {
                        let _ = call_tauri::<_, ()>("save_bookmark", &SaveBookmarkArgs { url: cur_url.clone(), title: cur_url }).await;
                        if let Ok(bm) = call_tauri::<_, Vec<BookmarkRecord>>("fetch_bookmarks", &EmptyArgs {}).await {
                            set_bookmarks.set(bm);
                        }
                    });
                }
            }><IconBookmark /></button>
        </div>

        <button class="icon-btn" on:click=move |_| navigate("caram://extensions".into(), true) title="Extensions"><IconExtension /></button>
        <button class="icon-btn" on:click=move |_| navigate("caram://downloads".into(), true) title="Downloads"><IconDownload /></button>
        <button class="icon-btn" on:click=move |_| navigate("caram://passwords".into(), true) title="Password Vault"><IconKey /></button>
        <button class="icon-btn" on:click=move |_| set_menu_open.set(!menu_open.get()) title="Settings & Menu"><IconMenu /></button>
    </div>
}
