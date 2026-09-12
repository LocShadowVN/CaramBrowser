mod icons;
mod tauri_ipc;
mod views;

use icons::*;
use leptos::*;
use serde::Serialize;
use shared::{AppConfig, BookmarkRecord, PageContentResponse, ShieldVerdict};
use tauri_ipc::call_tauri;
use views::{
    bookmarks::BookmarksView, downloads::DownloadsView, extensions::ExtensionsView,
    history::HistoryView, newtab::NewTabView, settings::SettingsView, vault::VaultView,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct ResolveArgs {
    raw: String,
    engine: String,
}

#[derive(Serialize)]
struct FetchPageArgs {
    url: String,
}

#[derive(Serialize)]
struct ShieldCheckArgs {
    target: String,
    host: String,
}

#[derive(Serialize)]
struct SaveBookmarkArgs {
    url: String,
    title: String,
}

#[derive(Serialize)]
struct IncStatArgs {
    count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageMode {
    Web,
    NewTab,
    Settings,
    History,
    Bookmarks,
    Downloads,
    Extensions,
    Vault,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub blocked_count: u32,
    pub history: Vec<String>,
    pub history_index: usize,
    pub page_mode: PageMode,
    pub html_content: Option<String>,
    pub is_loading: bool,
}

#[component]
fn App() -> impl IntoView {
    let (tab_counter, set_tab_counter) = create_signal(1u64);
    let (tabs, set_tabs) = create_signal(vec![BrowserTab {
        id: "tab_1".into(),
        url: "caram://newtab".into(),
        title: "New Tab".into(),
        blocked_count: 0,
        history: vec!["caram://newtab".into()],
        history_index: 0,
        page_mode: PageMode::NewTab,
        html_content: None,
        is_loading: false,
    }]);

    let (active_tab_id, set_active_tab_id) = create_signal("tab_1".to_string());
    let (omnibox_text, set_omnibox_text) = create_signal(String::new());
    let (shield_open, set_shield_open) = create_signal(false);
    let (menu_open, set_menu_open) = create_signal(false);
    let (bookmarks, set_bookmarks) = create_signal(Vec::<BookmarkRecord>::new());

    let (config, set_config) = create_signal(AppConfig::default());

    // Initialize configuration and bookmarks
    spawn_local(async move {
        if let Ok(cfg) = call_tauri::<_, AppConfig>("get_settings", &EmptyArgs {}).await {
            set_config.set(cfg);
        }
        if let Ok(bm) = call_tauri::<_, Vec<BookmarkRecord>>("fetch_bookmarks", &EmptyArgs {}).await {
            set_bookmarks.set(bm);
        }
    });

    // Theme synchronization
    create_effect(move |_| {
        let is_dark = config.get().dark_theme;
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(body) = doc.body() {
                let _ = body.set_attribute("data-theme", if is_dark { "dark" } else { "light" });
            }
        }
    });

    // Primary navigation handler
    let navigate = move |target_url: String, record_history: bool| {
        let engine = config.get().search_engine;
        spawn_local(async move {
            set_menu_open.set(false);
            set_shield_open.set(false);

            let cur_id = active_tab_id.get();
            let mut list = tabs.get();
            let tab_opt = list.iter_mut().find(|x| x.id == cur_id);
            if tab_opt.is_none() {
                return;
            }
            let tab = tab_opt.unwrap();

            let target = target_url.trim().to_string();

            match target.as_str() {
                "caram://newtab" => {
                    tab.url = target.clone();
                    tab.title = "New Tab".into();
                    tab.page_mode = PageMode::NewTab;
                    tab.html_content = None;
                    tab.is_loading = false;
                    if record_history {
                        tab.history.truncate(tab.history_index + 1);
                        tab.history.push(target);
                        tab.history_index = tab.history.len() - 1;
                    }
                    set_tabs.set(list);
                    set_omnibox_text.set(String::new());
                    return;
                }
                "caram://settings" => {
                    tab.url = target.clone();
                    tab.title = "Settings".into();
                    tab.page_mode = PageMode::Settings;
                    tab.html_content = None;
                    tab.is_loading = false;
                    if record_history {
                        tab.history.truncate(tab.history_index + 1);
                        tab.history.push(target.clone());
                        tab.history_index = tab.history.len() - 1;
                    }
                    set_tabs.set(list);
                    set_omnibox_text.set(target);
                    return;
                }
                "caram://history" => {
                    tab.url = target.clone();
                    tab.title = "History".into();
                    tab.page_mode = PageMode::History;
                    tab.html_content = None;
                    tab.is_loading = false;
                    if record_history {
                        tab.history.truncate(tab.history_index + 1);
                        tab.history.push(target.clone());
                        tab.history_index = tab.history.len() - 1;
                    }
                    set_tabs.set(list);
                    set_omnibox_text.set(target);
                    return;
                }
                "caram://bookmarks" => {
                    tab.url = target.clone();
                    tab.title = "Bookmarks".into();
                    tab.page_mode = PageMode::Bookmarks;
                    tab.html_content = None;
                    tab.is_loading = false;
                    if record_history {
                        tab.history.truncate(tab.history_index + 1);
                        tab.history.push(target.clone());
                        tab.history_index = tab.history.len() - 1;
                    }
                    set_tabs.set(list);
                    set_omnibox_text.set(target);
                    return;
                }
                "caram://downloads" => {
                    tab.url = target.clone();
                    tab.title = "Downloads".into();
                    tab.page_mode = PageMode::Downloads;
                    tab.html_content = None;
                    tab.is_loading = false;
                    if record_history {
                        tab.history.truncate(tab.history_index + 1);
                        tab.history.push(target.clone());
                        tab.history_index = tab.history.len() - 1;
                    }
                    set_tabs.set(list);
                    set_omnibox_text.set(target);
                    return;
                }
                "caram://extensions" => {
                    tab.url = target.clone();
                    tab.title = "Extensions".into();
                    tab.page_mode = PageMode::Extensions;
                    tab.html_content = None;
                    tab.is_loading = false;
                    if record_history {
                        tab.history.truncate(tab.history_index + 1);
                        tab.history.push(target.clone());
                        tab.history_index = tab.history.len() - 1;
                    }
                    set_tabs.set(list);
                    set_omnibox_text.set(target);
                    return;
                }
                "caram://passwords" => {
                    tab.url = target.clone();
                    tab.title = "Password Vault".into();
                    tab.page_mode = PageMode::Vault;
                    tab.html_content = None;
                    tab.is_loading = false;
                    if record_history {
                        tab.history.truncate(tab.history_index + 1);
                        tab.history.push(target.clone());
                        tab.history_index = tab.history.len() - 1;
                    }
                    set_tabs.set(list);
                    set_omnibox_text.set(target);
                    return;
                }
                _ => {}
            }

            let resolved: String = call_tauri("resolve_url", &ResolveArgs { raw: target, engine }).await.unwrap_or_default();
            tab.url = resolved.clone();
            tab.title = resolved.clone();
            tab.page_mode = PageMode::Web;
            tab.is_loading = true;
            if record_history {
                tab.history.truncate(tab.history_index + 1);
                tab.history.push(resolved.clone());
                tab.history_index = tab.history.len() - 1;
            }
            set_tabs.set(list);
            set_omnibox_text.set(resolved.clone());

            let res: Result<PageContentResponse, _> = call_tauri("fetch_web_page", &FetchPageArgs { url: resolved.clone() }).await;
            let mut list_after = tabs.get();
            if let Some(t) = list_after.iter_mut().find(|x| x.id == cur_id) {
                t.is_loading = false;
                match res {
                    Ok(resp) => {
                        t.url = resp.final_url.clone();
                        t.title = resp.title;
                        t.html_content = Some(resp.html);
                        t.blocked_count += resp.blocked_count;
                        set_omnibox_text.set(resp.final_url);
                    }
                    Err(err) => {
                        t.title = "Failed to Load".into();
                        t.html_content = Some(format!(r#"
                            <!DOCTYPE html><html><head><meta charset="utf-8">
                            <style>body {{ background:#0e1013; color:#e6e8eb; font-family:sans-serif; display:flex; align-items:center; justify-content:center; height:100vh; margin:0; }} .box {{ background:#16181d; border:1px solid #ef4444; border-radius:8px; padding:24px; max-width:440px; text-align:center; }}</style>
                            </head><body><div class="box"><h2 style="color:#ef4444;">Unable to connect</h2><p style="color:#8c929d; font-size:13px;">{}</p></div></body></html>
                        "#, err));
                    }
                }
            }
            set_tabs.set(list_after);
        });
    };

    // Install global message & shortcut listener
    let nav_msg = navigate;
    create_effect(move |_| {
        if let Some(window) = web_sys::window() {
            let nav_cb = nav_msg;
            let tabs_cb = tabs;
            let set_tabs_cb = set_tabs;
            let active_id_cb = active_tab_id;

            let msg_closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                if let Ok(parsed) = serde_wasm_bindgen::from_value::<serde_json::Value>(event.data()) {
                    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match msg_type {
                        "CARAM_NAVIGATE" => {
                            if let Some(target) = parsed.get("url").and_then(|v| v.as_str()) {
                                nav_cb(target.to_string(), true);
                            }
                        }
                        "CARAM_METADATA" => {
                            let cur = active_id_cb.get();
                            let mut list = tabs_cb.get();
                            if let Some(t) = list.iter_mut().find(|x| x.id == cur) {
                                if let Some(title) = parsed.get("title").and_then(|v| v.as_str()) {
                                    if !title.is_empty() {
                                        t.title = title.to_string();
                                    }
                                }
                            }
                            set_tabs_cb.set(list);
                        }
                        "CARAM_SHIELD_BLOCK" => {
                            let count = parsed.get("count").and_then(|v| v.as_u64()).unwrap_or(1);
                            let cur = active_id_cb.get();
                            let mut list = tabs_cb.get();
                            if let Some(t) = list.iter_mut().find(|x| x.id == cur) {
                                t.blocked_count += count as u32;
                            }
                            set_tabs_cb.set(list);
                            spawn_local(async move {
                                let _ = call_tauri::<_, ()>("increment_blocked_stat", &IncStatArgs { count }).await;
                            });
                        }
                        _ => {}
                    }
                }
            }) as Box<dyn FnMut(web_sys::MessageEvent)>);

            let _ = window.add_event_listener_with_callback("message", msg_closure.as_ref().unchecked_ref());
            msg_closure.forget();
        }
    });

    view! {
        <div class="browser-shell">
            <header class="tabs-strip">
                <div class="tabs-list">
                    {move || tabs.get().into_iter().map(|tab| {
                        let id = tab.id.clone();
                        let id_del = tab.id.clone();
                        let active = tab.id == active_tab_id.get();
                        let is_loading = tab.is_loading;
                        let tab_url = tab.url.clone();
                        view! {
                            <div class=format!("tab-chip {}", if active { "active" } else { "" }) on:click=move |_| {
                                set_active_tab_id.set(id.clone());
                                set_omnibox_text.set(if tab_url == "caram://newtab" { String::new() } else { tab_url.clone() });
                            }>
                                {if is_loading {
                                    view! { <div class="tab-spinner"></div> }.into_view()
                                } else {
                                    view! { <div style="display:none;"></div> }.into_view()
                                }}
                                <span>{tab.title}</span>
                                <div class="btn-tab-close" on:click=move |ev| {
                                    ev.stop_propagation();
                                    let mut t_list = tabs.get();
                                    if t_list.len() > 1 {
                                        let del_index = t_list.iter().position(|x| x.id == id_del);
                                        t_list.retain(|x| x.id != id_del);
                                        if active_tab_id.get() == id_del {
                                            let next_idx = del_index.unwrap_or(1).saturating_sub(1);
                                            let next_tab = &t_list[next_idx];
                                            set_active_tab_id.set(next_tab.id.clone());
                                            set_omnibox_text.set(if next_tab.url == "caram://newtab" { String::new() } else { next_tab.url.clone() });
                                        }
                                        set_tabs.set(t_list);
                                    }
                                }>
                                    <IconClose />
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
                <button class="icon-btn" on:click=move |_| {
                    let mut list = tabs.get();
                    let next_counter = tab_counter.get() + 1;
                    set_tab_counter.set(next_counter);
                    let new_id = format!("tab_{}", next_counter);
                    list.push(BrowserTab {
                        id: new_id.clone(),
                        url: "caram://newtab".into(),
                        title: "New Tab".into(),
                        blocked_count: 0,
                        history: vec!["caram://newtab".into()],
                        history_index: 0,
                        page_mode: PageMode::NewTab,
                        html_content: None,
                        is_loading: false,
                    });
                    set_tabs.set(list);
                    set_active_tab_id.set(new_id);
                    set_omnibox_text.set(String::new());
                }>
                    <IconPlus />
                </button>
            </header>

            <div class="nav-bar">
                // Back Button
                <button
                    class="icon-btn"
                    disabled=move || {
                        let cur = active_tab_id.get();
                        let list = tabs.get();
                        list.into_iter().find(|t| t.id == cur).map(|t| t.history_index == 0).unwrap_or(true)
                    }
                    on:click=move |_| {
                        let cur = active_tab_id.get();
                        let mut list = tabs.get();
                        if let Some(tab) = list.iter_mut().find(|t| t.id == cur) {
                            if tab.history_index > 0 {
                                tab.history_index -= 1;
                                let prev = tab.history[tab.history_index].clone();
                                drop(list);
                                navigate(prev, false);
                            }
                        }
                    }
                ><IconBack /></button>

                // Forward Button
                <button
                    class="icon-btn"
                    disabled=move || {
                        let cur = active_tab_id.get();
                        let list = tabs.get();
                        list.into_iter().find(|t| t.id == cur).map(|t| t.history_index + 1 >= t.history.len()).unwrap_or(true)
                    }
                    on:click=move |_| {
                        let cur = active_tab_id.get();
                        let mut list = tabs.get();
                        if let Some(tab) = list.iter_mut().find(|t| t.id == cur) {
                            if tab.history_index + 1 < tab.history.len() {
                                tab.history_index += 1;
                                let next = tab.history[tab.history_index].clone();
                                drop(list);
                                navigate(next, false);
                            }
                        }
                    }
                ><IconForward /></button>

                // Reload Button
                <button
                    class="icon-btn"
                    on:click=move |_| {
                        let cur = active_tab_id.get();
                        let list = tabs.get();
                        if let Some(tab) = list.into_iter().find(|t| t.id == cur) {
                            navigate(tab.url, false);
                        }
                    }
                ><IconReload /></button>

                <div class="omnibox-box">
                    <div class="lock-icon"><IconLock /></div>
                    <input
                        type="text"
                        class="omnibox-input"
                        placeholder="Search web or enter address"
                        prop:value=omnibox_text
                        on:input=move |ev| set_omnibox_text.set(event_target_value(&ev))
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                navigate(omnibox_text.get(), true);
                            }
                        }
                    />

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

            <div class="bookmarks-strip">
                {move || bookmarks.get().into_iter().map(|b| {
                    let u = b.url.clone();
                    view! {
                        <span class="bookmark-item" on:click=move |_| navigate(u.clone(), true)>
                            {b.title}
                        </span>
                    }
                }).collect_view()}
            </div>

            {move || if shield_open.get() {
                view! {
                    <div class="shield-flyout">
                        <div class="flyout-head">
                            <strong>"Caram Shield Core"</strong>
                            <span style="color:var(--accent-shield); font-size:12px;">"Active Protection"</span>
                        </div>
                        <div class="flyout-stat">
                            <div class="num">{move || {
                                let cur = active_tab_id.get();
                                tabs.get().into_iter().find(|t| t.id == cur).map(|x| x.blocked_count).unwrap_or(0)
                            }}</div>
                            <span style="font-size:11px; color:var(--text-secondary)">"Trackers & Ads Intercepted"</span>
                        </div>
                        <div style="font-size:12px; color:var(--text-secondary); text-align:center;">
                            "Brave-grade Bloom filter and tokenized pattern trie."
                        </div>
                    </div>
                }
            } else {
                view! { <div style="display:none;"></div> }
            }}

            {move || if menu_open.get() {
                view! {
                    <div class="hamburger-menu">
                        <div class="menu-item" on:click=move |_| navigate("caram://newtab".into(), true)>"New Tab (Ctrl+T)"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://history".into(), true)>"History (Ctrl+H)"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://downloads".into(), true)>"Downloads"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://bookmarks".into(), true)>"Bookmarks"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://extensions".into(), true)>"Extensions"</div>
                        <div class="menu-divider"></div>
                        <div class="menu-item" on:click=move |_| navigate("caram://passwords".into(), true)>"Passwords (Vault)"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://settings".into(), true)>"Settings"</div>
                        <div class="menu-divider"></div>
                        <div class="menu-item" on:click=move |_| {
                            spawn_local(async move {
                                let _ = call_tauri::<_, ()>("toggle_devtools", &EmptyArgs {}).await;
                            });
                        }>"Developer Tools (F12)"</div>
                    </div>
                }
            } else {
                view! { <div style="display:none;"></div> }
            }}

            <main class="viewport-body">
                {move || {
                    let cur_id = active_tab_id.get();
                    let current_tab = tabs.get().into_iter().find(|t| t.id == cur_id);
                    let mode = current_tab.as_ref().map(|t| t.page_mode.clone()).unwrap_or(PageMode::NewTab);

                    match mode {
                        PageMode::NewTab => view! { <NewTabView on_navigate=move |u| navigate(u, true) /> }.into_view(),
                        PageMode::Settings => view! { <SettingsView config=config set_config=set_config /> }.into_view(),
                        PageMode::History => view! { <HistoryView on_navigate=move |u| navigate(u, true) /> }.into_view(),
                        PageMode::Bookmarks => view! { <BookmarksView on_navigate=move |u| navigate(u, true) /> }.into_view(),
                        PageMode::Downloads => view! { <DownloadsView /> }.into_view(),
                        PageMode::Extensions => view! { <ExtensionsView /> }.into_view(),
                        PageMode::Vault => view! { <VaultView /> }.into_view(),
                        PageMode::Web => {
                            let html_doc = current_tab.and_then(|t| t.html_content).unwrap_or_else(|| "".into());
                            view! {
                                <iframe
                                    class="web-frame"
                                    srcdoc=html_doc
                                    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                                ></iframe>
                            }.into_view()
                        }
                    }
                }}
            </main>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}
