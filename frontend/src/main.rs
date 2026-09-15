mod icons;
mod tauri_ipc;
mod views;

use icons::*;
use leptos::*;
use serde::Serialize;
use shared::{AppConfig, BookmarkRecord};
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
struct OpenNativeTabArgs {
    tab_id: String,
    url: String,
}

#[derive(Serialize)]
struct SwitchTabArgs {
    active_tab_id: String,
    is_internal: bool,
    all_tab_ids: Vec<String>,
}

#[derive(Serialize)]
struct CloseNativeTabArgs {
    tab_id: String,
}

#[derive(Serialize)]
struct MenuExpandArgs {
    expanded: bool,
}

#[derive(Serialize)]
struct SaveBookmarkArgs {
    url: String,
    title: String,
}

#[derive(Serialize)]
struct GetSiteShieldArgs {
    domain: String,
}

#[derive(Serialize)]
struct ToggleSiteShieldArgs {
    domain: String,
    enabled: bool,
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
}

fn extract_domain(url_str: &str) -> String {
    if let Ok(u) = web_sys::Url::new(url_str) {
        let h = u.hostname();
        if !h.is_empty() {
            return h;
        }
    }
    if url_str.starts_with("caram://") {
        return "Caram System".into();
    }
    url_str.to_string()
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
    }]);

    let (active_tab_id, set_active_tab_id) = create_signal("tab_1".to_string());
    let (omnibox_text, set_omnibox_text) = create_signal(String::new());
    let (shield_open, set_shield_open) = create_signal(false);
    let (menu_open, set_menu_open) = create_signal(false);
    let (bookmarks, set_bookmarks) = create_signal(Vec::<BookmarkRecord>::new());
    let (current_site_shield, set_current_site_shield) = create_signal(true);

    let (config, set_config) = create_signal(AppConfig::default());

    spawn_local(async move {
        if let Ok(cfg) = call_tauri::<_, AppConfig>("get_settings", &EmptyArgs {}).await {
            set_config.set(cfg);
        }
        if let Ok(bm) = call_tauri::<_, Vec<BookmarkRecord>>("fetch_bookmarks", &EmptyArgs {}).await {
            set_bookmarks.set(bm);
        }
    });

    create_effect(move |_| {
        let is_dark = config.get().dark_theme;
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(body) = doc.body() {
                let _ = body.set_attribute("data-theme", if is_dark { "dark" } else { "light" });
            }
        }
    });

    create_effect(move |_| {
        let open = shield_open.get() || menu_open.get();
        spawn_local(async move {
            let _ = call_tauri::<_, ()>("expand_ui_for_menu", &MenuExpandArgs { expanded: open }).await;
        });
    });

    let sync_site_shield_status = move |target_url: &str| {
        let domain = extract_domain(target_url);
        if !domain.starts_with("Caram") && !domain.is_empty() {
            spawn_local(async move {
                if let Ok(enabled) = call_tauri::<_, bool>("get_site_shield", &GetSiteShieldArgs { domain }).await {
                    set_current_site_shield.set(enabled);
                }
            });
        }
    };

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

            let is_internal_route = match target.as_str() {
                "caram://newtab" => {
                    tab.url = target.clone();
                    tab.title = "New Tab".into();
                    tab.page_mode = PageMode::NewTab;
                    true
                }
                "caram://settings" => {
                    tab.url = target.clone();
                    tab.title = "Settings".into();
                    tab.page_mode = PageMode::Settings;
                    true
                }
                "caram://history" => {
                    tab.url = target.clone();
                    tab.title = "History".into();
                    tab.page_mode = PageMode::History;
                    true
                }
                "caram://bookmarks" => {
                    tab.url = target.clone();
                    tab.title = "Bookmarks".into();
                    tab.page_mode = PageMode::Bookmarks;
                    true
                }
                "caram://downloads" => {
                    tab.url = target.clone();
                    tab.title = "Downloads".into();
                    tab.page_mode = PageMode::Downloads;
                    true
                }
                "caram://extensions" => {
                    tab.url = target.clone();
                    tab.title = "Extensions".into();
                    tab.page_mode = PageMode::Extensions;
                    true
                }
                "caram://passwords" => {
                    tab.url = target.clone();
                    tab.title = "Password Vault".into();
                    tab.page_mode = PageMode::Vault;
                    true
                }
                _ => false,
            };

            if is_internal_route {
                if record_history {
                    tab.history.truncate(tab.history_index + 1);
                    tab.history.push(target.clone());
                    tab.history_index = tab.history.len() - 1;
                }
                set_tabs.set(list);
                set_omnibox_text.set(if target == "caram://newtab" { String::new() } else { target });

                let all_ids: Vec<String> = tabs.get().iter().map(|t| t.id.clone()).collect();
                let _ = call_tauri::<_, ()>("switch_tab_view", &SwitchTabArgs {
                    active_tab_id: cur_id,
                    is_internal: true,
                    all_tab_ids: all_ids,
                }).await;
                return;
            }

            let resolved: String = call_tauri("resolve_url", &ResolveArgs { raw: target.clone(), engine })
                .await
                .unwrap_or(target);

            tab.url = resolved.clone();
            tab.title = resolved.clone();
            tab.page_mode = PageMode::Web;
            if record_history {
                tab.history.truncate(tab.history_index + 1);
                tab.history.push(resolved.clone());
                tab.history_index = tab.history.len() - 1;
            }
            set_tabs.set(list);
            set_omnibox_text.set(resolved.clone());
            sync_site_shield_status(&resolved);

            let _ = call_tauri::<_, ()>("open_native_tab", &OpenNativeTabArgs {
                tab_id: cur_id,
                url: resolved,
            }).await;
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
                                let id_c = id.clone();
                                set_active_tab_id.set(id_c.clone());
                                let list = tabs.get();
                                let is_int = list.iter().find(|t| t.id == id_c).map(|t| t.url.starts_with("caram://")).unwrap_or(true);
                                let cur_url = list.iter().find(|t| t.id == id_c).map(|t| t.url.clone()).unwrap_or_default();
                                set_omnibox_text.set(if cur_url == "caram://newtab" { String::new() } else { cur_url.clone() });
                                sync_site_shield_status(&cur_url);
                                let all_ids: Vec<String> = list.iter().map(|t| t.id.clone()).collect();
                                spawn_local(async move {
                                    let _ = call_tauri::<_, ()>("switch_tab_view", &SwitchTabArgs {
                                        active_tab_id: id_c,
                                        is_internal: is_int,
                                        all_tab_ids: all_ids,
                                    }).await;
                                });
                            }>
                                <span>{tab.title}</span>
                                <div class="btn-tab-close" on:click=move |ev| {
                                    ev.stop_propagation();
                                    let mut t_list = tabs.get();
                                    if t_list.len() > 1 {
                                        let del_id = id_del.clone();
                                        let del_index = t_list.iter().position(|x| x.id == del_id);
                                        t_list.retain(|x| x.id != del_id);
                                        if active_tab_id.get() == del_id {
                                            let next_idx = del_index.unwrap_or(1).saturating_sub(1);
                                            let next_tab = &t_list[next_idx];
                                            set_active_tab_id.set(next_tab.id.clone());
                                            set_omnibox_text.set(if next_tab.url == "caram://newtab" { String::new() } else { next_tab.url.clone() });
                                        }
                                        set_tabs.set(t_list);
                                        spawn_local(async move {
                                            let _ = call_tauri::<_, ()>("close_native_tab", &CloseNativeTabArgs { tab_id: del_id }).await;
                                        });
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
                    });
                    set_tabs.set(list);
                    set_active_tab_id.set(new_id.clone());
                    set_omnibox_text.set(String::new());

                    let all_ids: Vec<String> = tabs.get().iter().map(|t| t.id.clone()).collect();
                    spawn_local(async move {
                        let _ = call_tauri::<_, ()>("switch_tab_view", &SwitchTabArgs {
                            active_tab_id: new_id,
                            is_internal: true,
                            all_tab_ids: all_ids,
                        }).await;
                    });
                }>
                    <IconPlus />
                </button>
            </header>

            <div class="nav-bar">
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
                                set_tabs.set(list);
                                navigate(prev, false);
                            }
                        }
                    }
                ><IconBack /></button>

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
                                set_tabs.set(list);
                                navigate(next, false);
                            }
                        }
                    }
                ><IconForward /></button>

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
                        placeholder="Search web or enter address (Auto-Cleaned & De-AMP)"
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

            // BRAVE SHIELD CONTROLLER FLYOUT
            {move || if shield_open.get() {
                let cur_url = omnibox_text.get();
                let domain = extract_domain(&cur_url);
                let dom_for_toggle = domain.clone();
                let is_site_enabled = current_site_shield.get();
                view! {
                    <div class="shield-flyout">
                        <div class="flyout-head">
                            <strong>"Caram Shield Core"</strong>
                            <span class="shield-status-badge" style=format!("color:{}", if is_site_enabled { "var(--accent-shield)" } else { "var(--danger)" })>
                                {if is_site_enabled { "Shields UP" } else { "Shields DOWN" }}
                            </span>
                        </div>

                        <div class="shield-site-box">
                            <div class="site-name">{domain}</div>
                            <label class="switch">
                                <input
                                    type="checkbox"
                                    checked=is_site_enabled
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        set_current_site_shield.set(checked);
                                        let d = dom_for_toggle.clone();
                                        spawn_local(async move {
                                            let _ = call_tauri::<_, ()>("toggle_site_shield", &ToggleSiteShieldArgs {
                                                domain: d,
                                                enabled: checked,
                                            }).await;
                                        });
                                    }
                                />
                                <span class="slider round"></span>
                            </label>
                        </div>

                        <div class="flyout-stat">
                            <div class="num">{move || {
                                let cur = active_tab_id.get();
                                tabs.get().into_iter().find(|t| t.id == cur).map(|x| x.blocked_count).unwrap_or(0)
                            }}</div>
                            <span style="font-size:11px; color:var(--text-secondary)">"Trackers, Ads & Cookies Neutralized"</span>
                        </div>
                        <div style="font-size:11px; color:var(--text-secondary); text-align:center; margin-top:8px;">
                            "Farbling Anti-Fingerprinting Active | Clean URLs"
                        </div>
                    </div>
                }
            } else {
                view! { <div style="display:none;"></div> }
            }}

            {move || if menu_open.get() {
                view! {
                    <div class="hamburger-menu">
                        <div class="menu-item" on:click=move |_| navigate("caram://newtab".into(), true)>"New Tab"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://history".into(), true)>"History"</div>
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
                        PageMode::Web => view! {
                            <div style="width:100%; height:100%; background:transparent;"></div>
                        }.into_view(),
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
