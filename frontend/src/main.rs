mod components;
mod icons;
mod tauri_ipc;
mod views;

use icons::*;
use leptos::*;
use serde::Serialize;
use shared::{AppConfig, BookmarkRecord, ShieldVerdict};
use tauri_ipc::call_tauri;
use views::{
    bookmarks::BookmarksView, downloads::DownloadsView, extensions::ExtensionsView,
    history::HistoryView, newtab::NewTabView, settings::SettingsView, vault::VaultView,
};

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct ResolveArgs {
    raw: String,
    engine: String,
}

#[derive(Serialize)]
struct ShieldCheckArgs {
    target: String,
    host: String,
}

#[derive(Serialize)]
struct RecordHistoryArgs {
    url: String,
    title: String,
}

#[derive(Serialize)]
struct SaveBookmarkArgs {
    url: String,
    title: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub blocked_count: u32,
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

#[component]
fn App() -> impl IntoView {
    let (tabs, set_tabs) = create_signal(vec![BrowserTab {
        id: "tab_1".into(),
        url: "caram://newtab".into(),
        title: "New Tab".into(),
        blocked_count: 0,
    }]);

    let (active_tab_id, set_active_tab_id) = create_signal("tab_1".to_string());
    let (page_mode, set_page_mode) = create_signal(PageMode::NewTab);
    let (omnibox_text, set_omnibox_text) = create_signal(String::new());
    let (shield_open, set_shield_open) = create_signal(false);
    let (menu_open, set_menu_open) = create_signal(false);
    let (bookmarks, set_bookmarks) = create_signal(Vec::<BookmarkRecord>::new());

    let (config, set_config) = create_signal(AppConfig {
        search_engine: "https://duckduckgo.com/?q=".into(),
        shield_level: "Standard".into(),
        doh_provider: "Cloudflare".into(),
        custom_doh_url: "https://cloudflare-dns.com/dns-query".into(),
        download_path: "/tmp".into(),
        dev_mode_extensions: true,
        dark_theme: true,
    });

    spawn_local(async move {
        if let Ok(cfg) = call_tauri::<_, AppConfig>("get_settings", &EmptyArgs {}).await {
            set_config.set(cfg);
        }
        if let Ok(bm) = call_tauri::<_, Vec<BookmarkRecord>>("fetch_bookmarks", &EmptyArgs {}).await {
            set_bookmarks.set(bm);
        }
    });

    let navigate = move |target_url: String| {
        let engine = config.get().search_engine;
        spawn_local(async move {
            set_menu_open.set(false);
            set_shield_open.set(false);

            match target_url.as_str() {
                "caram://newtab" => {
                    set_page_mode.set(PageMode::NewTab);
                    set_omnibox_text.set(String::new());
                    return;
                }
                "caram://settings" => {
                    set_page_mode.set(PageMode::Settings);
                    set_omnibox_text.set("caram://settings".into());
                    return;
                }
                "caram://history" => {
                    set_page_mode.set(PageMode::History);
                    set_omnibox_text.set("caram://history".into());
                    return;
                }
                "caram://bookmarks" => {
                    set_page_mode.set(PageMode::Bookmarks);
                    set_omnibox_text.set("caram://bookmarks".into());
                    return;
                }
                "caram://downloads" => {
                    set_page_mode.set(PageMode::Downloads);
                    set_omnibox_text.set("caram://downloads".into());
                    return;
                }
                "caram://extensions" => {
                    set_page_mode.set(PageMode::Extensions);
                    set_omnibox_text.set("caram://extensions".into());
                    return;
                }
                "caram://passwords" => {
                    set_page_mode.set(PageMode::Vault);
                    set_omnibox_text.set("caram://passwords".into());
                    return;
                }
                _ => {}
            }

            let resolved: String = call_tauri("resolve_url", &ResolveArgs { raw: target_url, engine }).await.unwrap_or_default();
            let verdict: Result<ShieldVerdict, _> = call_tauri("check_shield", &ShieldCheckArgs {
                target: resolved.clone(),
                host: resolved.clone(),
            }).await;

            let cur_id = active_tab_id.get();
            let mut list = tabs.get();
            if let Some(t) = list.iter_mut().find(|x| x.id == cur_id) {
                t.url = resolved.clone();
                t.title = resolved.clone();
                if let Ok(v) = verdict {
                    if v.blocked { t.blocked_count += 1; }
                }
            }
            set_tabs.set(list);
            set_omnibox_text.set(resolved.clone());
            set_page_mode.set(PageMode::Web);

            let _ = call_tauri::<_, ()>("record_history", &RecordHistoryArgs { url: resolved.clone(), title: resolved }).await;
        });
    };

    view! {
        <div class="browser-shell">
            <header class="tabs-strip">
                <div class="tabs-list">
                    {move || tabs.get().into_iter().map(|tab| {
                        let id = tab.id.clone();
                        let id_del = tab.id.clone();
                        let active = tab.id == active_tab_id.get();
                        view! {
                            <div class=format!("tab-chip {}", if active { "active" } else { "" }) on:click=move |_| {
                                set_active_tab_id.set(id.clone());
                                set_page_mode.set(PageMode::Web);
                            }>
                                <span>{tab.title}</span>
                                <div class="btn-tab-close" on:click=move |ev| {
                                    ev.stop_propagation();
                                    let mut t_list = tabs.get();
                                    if t_list.len() > 1 {
                                        t_list.retain(|x| x.id != id_del);
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
                    let new_id = format!("tab_{}", list.len() + 1);
                    list.push(BrowserTab {
                        id: new_id.clone(),
                        url: "caram://newtab".into(),
                        title: "New Tab".into(),
                        blocked_count: 0,
                    });
                    set_tabs.set(list);
                    set_active_tab_id.set(new_id);
                    set_page_mode.set(PageMode::NewTab);
                    set_omnibox_text.set(String::new());
                }>
                    <IconPlus />
                </button>
            </header>

            <div class="nav-bar">
                <button class="icon-btn"><IconBack /></button>
                <button class="icon-btn"><IconForward /></button>
                <button class="icon-btn"><IconReload /></button>

                <div class="omnibox-box">
                    <div class="lock-icon"><IconLock /></div>
                    <input
                        type="text"
                        class="omnibox-input"
                        placeholder="Search or enter web address"
                        prop:value=omnibox_text
                        on:input=move |ev| set_omnibox_text.set(event_target_value(&ev))
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                navigate(omnibox_text.get());
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
                        spawn_local(async move {
                            let _ = call_tauri::<_, ()>("save_bookmark", &SaveBookmarkArgs { url: cur_url.clone(), title: cur_url }).await;
                            if let Ok(bm) = call_tauri::<_, Vec<BookmarkRecord>>("fetch_bookmarks", &EmptyArgs {}).await {
                                set_bookmarks.set(bm);
                            }
                        });
                    }><IconBookmark /></button>
                </div>

                <button class="icon-btn" on:click=move |_| navigate("caram://extensions".into()) title="Extensions"><IconExtension /></button>
                <button class="icon-btn" on:click=move |_| navigate("caram://downloads".into()) title="Downloads"><IconDownload /></button>
                <button class="icon-btn" on:click=move |_| navigate("caram://passwords".into()) title="Password Vault"><IconKey /></button>
                <button class="icon-btn" on:click=move |_| set_menu_open.set(!menu_open.get()) title="Settings & More"><IconMenu /></button>
            </div>

            <div class="bookmarks-strip">
                {move || bookmarks.get().into_iter().map(|b| {
                    let u = b.url.clone();
                    view! {
                        <span class="bookmark-item" on:click=move |_| navigate(u.clone())>
                            {b.title}
                        </span>
                    }
                }).collect_view()}
            </div>

            {move || if shield_open.get() {
                view! {
                    <div class="shield-flyout">
                        <div class="flyout-head">
                            <strong>"Caram Shield"</strong>
                            <span style="color:var(--accent-shield); font-size:12px;">"Active"</span>
                        </div>
                        <div class="flyout-stat">
                            <div class="num">{move || {
                                let cur = active_tab_id.get();
                                tabs.get().into_iter().find(|t| t.id == cur).map(|x| x.blocked_count).unwrap_or(0)
                            }}</div>
                            <span style="font-size:11px; color:var(--text-secondary)">"Trackers & Ads Blocked"</span>
                        </div>
                    </div>
                }
            } else {
                view! { <div style="display:none;"></div> }
            }}

            {move || if menu_open.get() {
                view! {
                    <div class="hamburger-menu">
                        <div class="menu-item" on:click=move |_| navigate("caram://newtab".into())>"New Tab"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://history".into())>"History"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://downloads".into())>"Downloads"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://bookmarks".into())>"Bookmarks"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://extensions".into())>"Extensions"</div>
                        <div class="menu-divider"></div>
                        <div class="menu-item" on:click=move |_| navigate("caram://passwords".into())>"Passwords (Vault)"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://settings".into())>"Settings"</div>
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
                {move || match page_mode.get() {
                    PageMode::NewTab => view! { <NewTabView on_navigate=navigate /> }.into_view(),
                    PageMode::Settings => view! { <SettingsView config=config set_config=set_config /> }.into_view(),
                    PageMode::History => view! { <HistoryView /> }.into_view(),
                    PageMode::Bookmarks => view! { <BookmarksView on_navigate=navigate /> }.into_view(),
                    PageMode::Downloads => view! { <DownloadsView /> }.into_view(),
                    PageMode::Extensions => view! { <ExtensionsView /> }.into_view(),
                    PageMode::Vault => view! { <VaultView /> }.into_view(),
                    PageMode::Web => {
                        let cur_id = active_tab_id.get();
                        let tab = tabs.get().into_iter().find(|t| t.id == cur_id);
                        let target_src = tab.map(|t| t.url).unwrap_or_else(|| "about:blank".into());
                        view! {
                            <iframe class="web-frame" src=target_src sandbox="allow-scripts allow-same-origin allow-forms allow-popups"></iframe>
                        }.into_view()
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
