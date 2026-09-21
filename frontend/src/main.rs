mod icons;
mod tauri_ipc;
mod views;

use icons::*;
use leptos::*;
use serde::{Deserialize, Serialize};
use shared::{AppConfig, BookmarkRecord, DownloadProgressPayload, SiteCredential};
use tauri_ipc::call_tauri;
use views::{
    bookmarks::BookmarksView, downloads::DownloadsView, extensions::ExtensionsView,
    history::HistoryView, newtab::NewTabView, settings::SettingsView, vault::VaultView,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

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
    is_incognito: bool,
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
struct SnoozeTabArgs {
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

#[derive(Serialize)]
struct CheckVaultDomainArgs {
    domain: String,
}

#[derive(Serialize)]
struct ExecuteAutofillArgs {
    username: String,
    secret: String,
}

#[derive(Serialize)]
struct FindArgs {
    query: String,
    forward: bool,
}

#[derive(Serialize)]
struct ReloadArgs {
    hard: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PageNavigationState {
    pub tab_id: String,
    pub url: String,
    pub title: Option<String>,
    pub is_loading: bool,
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
    pub is_snoozed: bool,
    pub is_incognito: bool,
    pub is_loading: bool,
    pub last_active: f64,
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
        is_snoozed: false,
        is_incognito: false,
        is_loading: false,
        last_active: js_sys::Date::now(),
    }]);

    let (active_tab_id, set_active_tab_id) = create_signal("tab_1".to_string());
    let (omnibox_text, set_omnibox_text) = create_signal(String::new());
    let (shield_open, set_shield_open) = create_signal(false);
    let (menu_open, set_menu_open) = create_signal(false);
    let (find_open, set_find_open) = create_signal(false);
    let (find_query, set_find_query) = create_signal(String::new());

    let (bookmarks, set_bookmarks) = create_signal(Vec::<BookmarkRecord>::new());
    let (current_site_shield, set_current_site_shield) = create_signal(true);
    let (available_credentials, set_available_credentials) = create_signal(Vec::<SiteCredential>::new());
    let (active_download, set_active_download) = create_signal(Option::<DownloadProgressPayload>::None);

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

    // 1. ĐỒNG BỘ TIẾN TRÌNH TẢI IDM
    spawn_local(async move {
        let cb = Closure::wrap(Box::new(move |event_obj: JsValue| {
            if let Ok(payload_val) = js_sys::Reflect::get(&event_obj, &JsValue::from_str("payload")) {
                if let Ok(prog) = serde_wasm_bindgen::from_value::<DownloadProgressPayload>(payload_val) {
                    set_active_download.set(Some(prog));
                }
            }
        }) as Box<dyn FnMut(JsValue)>);
        let _ = tauri_ipc::listen("download-progress", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });

    // 2. ĐỒNG BỘ HAI CHIỀU (URL, TIÊU ĐỀ, TRẠNG THÁI LOADING BAR TỪ WEBKIT)
    spawn_local(async move {
        let cb = Closure::wrap(Box::new(move |event_obj: JsValue| {
            if let Ok(payload_val) = js_sys::Reflect::get(&event_obj, &JsValue::from_str("payload")) {
                if let Ok(state) = serde_wasm_bindgen::from_value::<PageNavigationState>(payload_val) {
                    let mut list = tabs.get();
                    if let Some(tab) = list.iter_mut().find(|t| t.id == state.tab_id) {
                        tab.is_loading = state.is_loading;
                        if !state.url.is_empty() {
                            tab.url = state.url.clone();
                        }
                        if let Some(t) = state.title {
                            if !t.is_empty() {
                                tab.title = t;
                            }
                        }
                    }
                    set_tabs.set(list);

                    // Cập nhật thanh omnibox nếu tab đó đang là tab active
                    if active_tab_id.get() == state.tab_id && !state.url.starts_with("caram://") {
                        set_omnibox_text.set(state.url);
                    }
                }
            }
        }) as Box<dyn FnMut(JsValue)>);
        let _ = tauri_ipc::listen("tab-navigation-state", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });

    // 3. SMART TAB SNOOZER (Ru ngủ tab sau 10 phút)
    spawn_local(async move {
        loop {
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                if let Some(w) = web_sys::window() {
                    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 30_000);
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

            let now = js_sys::Date::now();
            let cur_active = active_tab_id.get();
            let mut list = tabs.get();
            let mut changed = false;

            for t in list.iter_mut() {
                if t.id != cur_active && !t.is_snoozed && !t.url.starts_with("caram://") && (now - t.last_active > 600_000.0) {
                    t.is_snoozed = true;
                    changed = true;
                    let id_c = t.id.clone();
                    spawn_local(async move {
                        let _ = call_tauri::<_, ()>("snooze_tab", &SnoozeTabArgs { tab_id: id_c }).await;
                    });
                }
            }

            if changed {
                set_tabs.set(list);
            }
        }
    });

    let sync_site_state = move |target_url: &str| {
        let domain = extract_domain(target_url);
        if !domain.starts_with("Caram") && !domain.is_empty() {
            let d1 = domain.clone();
            let d2 = domain;
            spawn_local(async move {
                if let Ok(enabled) = call_tauri::<_, bool>("get_site_shield", &GetSiteShieldArgs { domain: d1 }).await {
                    set_current_site_shield.set(enabled);
                }
                if let Ok(creds) = call_tauri::<_, Vec<SiteCredential>>("check_vault_credentials_for_domain", &CheckVaultDomainArgs { domain: d2 }).await {
                    set_available_credentials.set(creds);
                }
            });
        } else {
            set_available_credentials.set(Vec::new());
        }
    };

    let navigate = move |target_url: String, record_history: bool| {
        let engine = config.get().search_engine;
        spawn_local(async move {
            set_menu_open.set(false);
            set_shield_open.set(false);
            set_find_open.set(false);

            let cur_id = active_tab_id.get();
            let mut list = tabs.get();
            let tab_opt = list.iter_mut().find(|x| x.id == cur_id);
            if tab_opt.is_none() {
                return;
            }
            let tab = tab_opt.unwrap();
            tab.last_active = js_sys::Date::now();
            tab.is_snoozed = false;
            let incognito = tab.is_incognito;

            let target = target_url.trim().to_string();

            let is_internal_route = match target.as_str() {
                "caram://newtab" => {
                    tab.url = target.clone();
                    tab.title = if incognito { "Incognito Tab".into() } else { "New Tab".into() };
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
                if record_history && !incognito {
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
            tab.is_loading = true;

            if record_history && !incognito {
                tab.history.truncate(tab.history_index + 1);
                tab.history.push(resolved.clone());
                tab.history_index = tab.history.len() - 1;
            }
            set_tabs.set(list);
            set_omnibox_text.set(resolved.clone());
            sync_site_state(&resolved);

            let _ = call_tauri::<_, ()>("open_native_tab", &OpenNativeTabArgs {
                tab_id: cur_id,
                url: resolved,
                is_incognito: incognito,
            }).await;
        });
    };

    let create_new_tab = move |incognito: bool| {
        let mut list = tabs.get();
        let next_counter = tab_counter.get() + 1;
        set_tab_counter.set(next_counter);
        let new_id = format!("tab_{}", next_counter);
        list.push(BrowserTab {
            id: new_id.clone(),
            url: "caram://newtab".into(),
            title: if incognito { "Incognito Tab".into() } else { "New Tab".into() },
            blocked_count: 0,
            history: vec!["caram://newtab".into()],
            history_index: 0,
            page_mode: PageMode::NewTab,
            is_snoozed: false,
            is_incognito: incognito,
            is_loading: false,
            last_active: js_sys::Date::now(),
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
    };

    // 4. HỆ THỐNG BẮT PHÍM TẮT TOÀN CỤC (Shortcuts Engine)
    {
        let window = web_sys::window().unwrap();
        let key_closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            let ctrl = e.ctrl_key() || e.meta_key();
            let key = e.key();

            if ctrl {
                match key.to_lowercase().as_str() {
                    "t" => {
                        e.prevent_default();
                        create_new_tab(e.shift_key()); // Ctrl+Shift+T mở tab ẩn danh
                    }
                    "w" => {
                        e.prevent_default();
                        let cur_id = active_tab_id.get();
                        let mut t_list = tabs.get();
                        if t_list.len() > 1 {
                            let del_index = t_list.iter().position(|x| x.id == cur_id);
                            t_list.retain(|x| x.id != cur_id);
                            let next_idx = del_index.unwrap_or(1).saturating_sub(1);
                            let next_tab = &t_list[next_idx];
                            set_active_tab_id.set(next_tab.id.clone());
                            set_omnibox_text.set(if next_tab.url == "caram://newtab" { String::new() } else { next_tab.url.clone() });
                            set_tabs.set(t_list);
                            spawn_local(async move {
                                let _ = call_tauri::<_, ()>("close_native_tab", &CloseNativeTabArgs { tab_id: cur_id }).await;
                            });
                        }
                    }
                    "l" => {
                        e.prevent_default();
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Some(input) = doc.query_selector(".omnibox-input").ok().flatten() {
                                if let Ok(el) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                    let _ = el.focus();
                                    el.select();
                                }
                            }
                        }
                    }
                    "r" => {
                        e.prevent_default();
                        let hard = e.shift_key();
                        spawn_local(async move {
                            let _ = call_tauri::<_, ()>("webview_reload", &ReloadArgs { hard }).await;
                        });
                    }
                    "f" => {
                        e.prevent_default();
                        set_find_open.set(!find_open.get());
                    }
                    "h" => {
                        e.prevent_default();
                        navigate("caram://history".into(), true);
                    }
                    "j" => {
                        e.prevent_default();
                        navigate("caram://downloads".into(), true);
                    }
                    "tab" => {
                        e.prevent_default();
                        let cur_id = active_tab_id.get();
                        let list = tabs.get();
                        if let Some(idx) = list.iter().position(|t| t.id == cur_id) {
                            let next_idx = if e.shift_key() {
                                if idx == 0 { list.len() - 1 } else { idx - 1 }
                            } else {
                                (idx + 1) % list.len()
                            };
                            let target_tab = &list[next_idx];
                            let next_id = target_tab.id.clone();
                            set_active_tab_id.set(next_id.clone());
                            set_omnibox_text.set(if target_tab.url == "caram://newtab" { String::new() } else { target_tab.url.clone() });
                        }
                    }
                    _ => {}
                }
            } else if key == "F5" {
                e.prevent_default();
                spawn_local(async move {
                    let _ = call_tauri::<_, ()>("webview_reload", &ReloadArgs { hard: false }).await;
                });
            } else if key == "Escape" {
                set_find_open.set(false);
                set_shield_open.set(false);
                set_menu_open.set(false);
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

        let _ = window.add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref());
        key_closure.forget();
    }

    let do_find = move |forward: bool| {
        let q = find_query.get();
        if !q.trim().is_empty() {
            spawn_local(async move {
                let _ = call_tauri::<_, bool>("find_in_page", &FindArgs { query: q, forward }).await;
            });
        }
    };

    view! {
        <div class="browser-shell">
            <header class="tabs-strip">
                <div class="tabs-list">
                    {move || tabs.get().into_iter().map(|tab| {
                        let id = tab.id.clone();
                        let id_del = tab.id.clone();
                        let id_snooze = tab.id.clone();
                        let active = tab.id == active_tab_id.get();
                        let snoozed = tab.is_snoozed;
                        let incognito = tab.is_incognito;
                        view! {
                            <div
                                class=format!("tab-chip {} {} {}", if active { "active" } else { "" }, if snoozed { "snoozed" } else { "" }, if incognito { "incognito" } else { "" })
                                on:click=move |_| {
                                    let id_c = id.clone();
                                    set_active_tab_id.set(id_c.clone());
                                    let mut list = tabs.get();
                                    let is_int = list.iter().find(|t| t.id == id_c).map(|t| t.url.starts_with("caram://")).unwrap_or(true);
                                    let cur_url = list.iter().find(|t| t.id == id_c).map(|t| t.url.clone()).unwrap_or_default();
                                    let was_snoozed = list.iter().find(|t| t.id == id_c).map(|t| t.is_snoozed).unwrap_or(false);
                                    let is_inc = list.iter().find(|t| t.id == id_c).map(|t| t.is_incognito).unwrap_or(false);

                                    if let Some(t) = list.iter_mut().find(|t| t.id == id_c) {
                                        t.last_active = js_sys::Date::now();
                                        t.is_snoozed = false;
                                    }
                                    set_tabs.set(list);

                                    set_omnibox_text.set(if cur_url == "caram://newtab" { String::new() } else { cur_url.clone() });
                                    sync_site_state(&cur_url);

                                    let all_ids: Vec<String> = tabs.get().iter().map(|t| t.id.clone()).collect();
                                    spawn_local(async move {
                                        if was_snoozed && !is_int {
                                            let _ = call_tauri::<_, ()>("open_native_tab", &OpenNativeTabArgs {
                                                tab_id: id_c.clone(),
                                                url: cur_url,
                                                is_incognito: is_inc,
                                            }).await;
                                        }
                                        let _ = call_tauri::<_, ()>("switch_tab_view", &SwitchTabArgs {
                                            active_tab_id: id_c,
                                            is_internal: is_int,
                                            all_tab_ids: all_ids,
                                        }).await;
                                    });
                                }
                            >
                                {if incognito {
                                    view! { <span style="margin-right:3px;">"🕶️"</span> }.into_view()
                                } else {
                                    view! { <span style="display:none;"></span> }.into_view()
                                }}
                                <span>{tab.title}</span>

                                {if !active && !snoozed && !tab.url.starts_with("caram://") {
                                    view! {
                                        <div
                                            class="btn-tab-snooze"
                                            title="Snooze tab to free RAM"
                                            on:click=move |ev| {
                                                ev.stop_propagation();
                                                let id_s = id_snooze.clone();
                                                let mut t_list = tabs.get();
                                                if let Some(t) = t_list.iter_mut().find(|x| x.id == id_s) {
                                                    t.is_snoozed = true;
                                                }
                                                set_tabs.set(t_list);
                                                spawn_local(async move {
                                                    let _ = call_tauri::<_, ()>("snooze_tab", &SnoozeTabArgs { tab_id: id_s }).await;
                                                });
                                            }
                                        >
                                            "💤"
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <div style="display:none;"></div> }.into_view()
                                }}

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

                <button class="icon-btn" title="New Tab (Ctrl+T)" on:click=move |_| create_new_tab(false)>
                    <IconPlus />
                </button>
            </header>

            <div class="nav-bar">
                <button
                    class="icon-btn"
                    title="Go Back"
                    on:click=move |_| {
                        let cur = active_tab_id.get();
                        let mut list = tabs.get();
                        if let Some(tab) = list.iter_mut().find(|t| t.id == cur) {
                            if tab.page_mode == PageMode::Web {
                                spawn_local(async move {
                                    let _ = call_tauri::<_, ()>("webview_go_back", &EmptyArgs {}).await;
                                });
                            } else if tab.history_index > 0 {
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
                    title="Go Forward"
                    on:click=move |_| {
                        let cur = active_tab_id.get();
                        let mut list = tabs.get();
                        if let Some(tab) = list.iter_mut().find(|t| t.id == cur) {
                            if tab.page_mode == PageMode::Web {
                                spawn_local(async move {
                                    let _ = call_tauri::<_, ()>("webview_go_forward", &EmptyArgs {}).await;
                                });
                            } else if tab.history_index + 1 < tab.history.len() {
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
                    title="Reload (Ctrl+R, Shift+R for Hard Reload)"
                    on:click=move |_| {
                        let cur = active_tab_id.get();
                        let list = tabs.get();
                        if let Some(tab) = list.into_iter().find(|t| t.id == cur) {
                            if tab.page_mode == PageMode::Web {
                                spawn_local(async move {
                                    let _ = call_tauri::<_, ()>("webview_reload", &ReloadArgs { hard: false }).await;
                                });
                            } else {
                                navigate(tab.url, false);
                            }
                        }
                    }
                ><IconReload /></button>

                <div class="omnibox-box">
                    <div class="lock-icon"><IconLock /></div>
                    <input
                        type="text"
                        class="omnibox-input"
                        placeholder="Search web or enter address (Ctrl+L to focus)"
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

                <button class="icon-btn" on:click=move |_| set_find_open.set(!find_open.get()) title="Find in Page (Ctrl+F)"><span style="font-weight:700; font-size:12px;">"🔍"</span></button>
                <button class="icon-btn" on:click=move |_| navigate("caram://extensions".into(), true) title="Extensions"><IconExtension /></button>
                <button class="icon-btn" on:click=move |_| navigate("caram://downloads".into(), true) title="Downloads (Ctrl+J)"><IconDownload /></button>
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

            // HỘP THOẠI TÌM KIẾM TRONG TRANG (FIND IN PAGE - CTRL + F)
            {move || if find_open.get() {
                view! {
                    <div class="find-bar">
                        <input
                            type="text"
                            placeholder="Find in page..."
                            prop:value=find_query
                            on:input=move |ev| set_find_query.set(event_target_value(&ev))
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Enter" {
                                    do_find(!ev.shift_key());
                                }
                            }
                        />
                        <button class="icon-btn" title="Previous" on:click=move |_| do_find(false)>"▲"</button>
                        <button class="icon-btn" title="Next" on:click=move |_| do_find(true)>"▼"</button>
                        <button class="icon-btn" title="Close" on:click=move |_| set_find_open.set(false)><IconClose /></button>
                    </div>
                }
            } else {
                view! { <div style="display:none;"></div> }
            }}

            // SHIELD CONTROLLER FLYOUT
            {move || if shield_open.get() {
                let cur_url = omnibox_text.get();
                let domain = extract_domain(&cur_url);
                let dom_for_toggle = domain.clone();
                let is_site_enabled = current_site_shield.get();
                let badge_style = if is_site_enabled { "color:var(--accent-shield)" } else { "color:var(--danger)" };
                view! {
                    <div class="shield-flyout">
                        <div class="flyout-head">
                            <strong>"Caram Shield Core"</strong>
                            <span class="shield-status-badge" style=badge_style>
                                {if is_site_enabled { "Shields UP" } else { "Shields DOWN" }}
                            </span>
                        </div>

                        <div class="shield-site-box">
                            <div class="site-name">{domain}</div>
                            <label class="switch">
                                <input
                                    type="checkbox"
                                    prop:checked=is_site_enabled
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

                        // Nút xoá sạch Cookie của trang này
                        <button
                            class="btn-action"
                            style="margin-top:12px; width:100%; background:var(--bg-tertiary); font-size:11px;"
                            on:click=move |_| {
                                spawn_local(async move {
                                    let _ = call_tauri::<_, ()>("clear_site_data", &EmptyArgs {}).await;
                                });
                            }
                        >
                            "Clear Cookies & Cache for this site"
                        </button>
                    </div>
                }
            } else {
                view! { <div style="display:none;"></div> }
            }}

            {move || if menu_open.get() {
                view! {
                    <div class="hamburger-menu">
                        <div class="menu-item" on:click=move |_| create_new_tab(false)>"New Tab (Ctrl+T)"</div>
                        <div class="menu-item" on:click=move |_| create_new_tab(true)>"New Incognito Tab (Ctrl+Shift+T)"</div>
                        <div class="menu-divider"></div>
                        <div class="menu-item" on:click=move |_| navigate("caram://history".into(), true)>"History (Ctrl+H)"</div>
                        <div class="menu-item" on:click=move |_| navigate("caram://downloads".into(), true)>"Downloads (Ctrl+J)"</div>
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

            // DOWNLOAD SHELF ĐA LUỒNG IDM
            {move || active_download.get().map(|prog| {
                view! {
                    <div class="download-shelf">
                        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:6px;">
                            <span style="font-weight:600; font-size:12px; max-width:170px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                                {prog.filename}
                            </span>
                            <div style="display:flex; align-items:center; gap:8px;">
                                <span style="font-size:11px; color:var(--accent); font-family:var(--mono);">
                                    {format!("{} Mbps ({} threads)", prog.speed_mbps, prog.threads)}
                                </span>
                                <button class="icon-btn" style="padding:2px; font-size:10px;" on:click=move |_| set_active_download.set(None)>
                                    <IconClose />
                                </button>
                            </div>
                        </div>
                        <div class="shelf-progress-bar">
                            <div class="shelf-progress-fill" style=format!("width: {}%", prog.progress_percent)></div>
                        </div>
                        <div style="display:flex; justify-content:space-between; margin-top:4px; font-size:10px; color:var(--text-secondary);">
                            <span>{format!("{:.1}%", prog.progress_percent)}</span>
                            <span>{prog.status}</span>
                        </div>
                    </div>
                }
            })}

            <main class="viewport-body">
                // THANH TIẾN TRÌNH TẢI TRANG (PAGE LOADING PROGRESS BAR)
                {move || {
                    let cur_id = active_tab_id.get();
                    let is_loading = tabs.get().into_iter().find(|t| t.id == cur_id).map(|t| t.is_loading).unwrap_or(false);
                    if is_loading {
                        view! { <div class="page-loading-bar"></div> }.into_view()
                    } else {
                        view! { <div style="display:none;"></div> }.into_view()
                    }
                }}

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
