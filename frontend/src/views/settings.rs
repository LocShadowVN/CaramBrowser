use crate::tauri_ipc::call_tauri;
use leptos::*;
use serde::Serialize;
use shared::{AppConfig, DnsTestResult};

#[derive(Serialize)]
struct SettingArgs {
    key: String,
    value: String,
}

#[derive(Serialize)]
struct DohArgs {
    url: String,
}

#[derive(Clone, PartialEq)]
enum SettingsTab {
    General,
    Shield,
    Dns,
    Appearance,
    About,
}

#[component]
pub fn SettingsView(config: ReadSignal<AppConfig>, set_config: WriteSignal<AppConfig>) -> impl IntoView {
    let (tab, set_tab) = create_signal(SettingsTab::General);
    let (doh_result, set_doh_result) = create_signal(String::new());
    let (doh_input, set_doh_input) = create_signal(config.get().custom_doh_url);

    let test_dns = move |_| {
        let endpoint = doh_input.get();
        set_doh_result.set("Testing latency...".into());
        spawn_local(async move {
            let res: Result<DnsTestResult, _> = call_tauri("test_doh", &DohArgs { url: endpoint.clone() }).await;
            match res {
                Ok(r) if r.success => {
                    set_doh_result.set(format!("Connected! Latency: {} ms (IP: {})", r.latency_ms, r.resolved_ip.unwrap_or_default()));
                    let _ = call_tauri::<_, ()>("update_setting", &SettingArgs { key: "custom_doh_url".into(), value: endpoint }).await;
                }
                Ok(r) => set_doh_result.set(format!("Failed: {}", r.error.unwrap_or_default())),
                Err(e) => set_doh_result.set(format!("Error: {}", e)),
            }
        });
    };

    view! {
        <div class="internal-view">
            <div class="settings-container">
                <aside class="settings-sidebar">
                    <button class=format!("sidebar-btn {}", if tab.get() == SettingsTab::General { "active" } else { "" }) on:click=move |_| set_tab.set(SettingsTab::General)>"General"</button>
                    <button class=format!("sidebar-btn {}", if tab.get() == SettingsTab::Shield { "active" } else { "" }) on:click=move |_| set_tab.set(SettingsTab::Shield)>"Caram Shield"</button>
                    <button class=format!("sidebar-btn {}", if tab.get() == SettingsTab::Dns { "active" } else { "" }) on:click=move |_| set_tab.set(SettingsTab::Dns)>"Secure DNS"</button>
                    <button class=format!("sidebar-btn {}", if tab.get() == SettingsTab::Appearance { "active" } else { "" }) on:click=move |_| set_tab.set(SettingsTab::Appearance)>"Appearance"</button>
                    <button class=format!("sidebar-btn {}", if tab.get() == SettingsTab::About { "active" } else { "" }) on:click=move |_| set_tab.set(SettingsTab::About)>"About Caram"</button>
                </aside>

                <main class="settings-content">
                    {move || match tab.get() {
                        SettingsTab::General => view! {
                            <div class="panel-card">
                                <h2>"General Preferences"</h2>
                                <div class="grid-form">
                                    <label>"Default Search Engine"</label>
                                    <select on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        let mut c = config.get();
                                        c.search_engine = val.clone();
                                        set_config.set(c);
                                        spawn_local(async move {
                                            let _ = call_tauri::<_, ()>("update_setting", &SettingArgs { key: "search_engine".into(), value: val }).await;
                                        });
                                    }>
                                        <option value="https://search.brave.com/search?q=" selected=move || config.get().search_engine.contains("brave")>"Brave Search (Default)"</option>
                                        <option value="https://duckduckgo.com/?q=" selected=move || config.get().search_engine.contains("duckduckgo")>"DuckDuckGo"</option>
                                        <option value="https://www.google.com/search?q=" selected=move || config.get().search_engine.contains("google")>"Google"</option>
                                    </select>

                                    <label>"Downloads Save Path"</label>
                                    <input
                                        type="text"
                                        prop:value=move || config.get().download_path
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            let mut c = config.get();
                                            c.download_path = val.clone();
                                            set_config.set(c);
                                            spawn_local(async move {
                                                let _ = call_tauri::<_, ()>("update_setting", &SettingArgs { key: "download_path".into(), value: val }).await;
                                            });
                                        }
                                    />
                                </div>
                            </div>
                        }.into_view(),

                        SettingsTab::Shield => view! {
                            <div class="panel-card">
                                <h2>"Caram Shield & Brave Engine Protection"</h2>
                                <div class="grid-form">
                                    <label>"Protection Level"</label>
                                    <select on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        let mut c = config.get();
                                        c.shield_level = val.clone();
                                        set_config.set(c);
                                        let val_clone = val.clone();
                                        spawn_local(async move {
                                            let _ = call_tauri::<_, ()>("update_setting", &SettingArgs { key: "shield_level".into(), value: val }).await;
                                            let _ = call_tauri::<_, ()>("set_shield_level", &SettingArgs { key: "level".into(), value: val_clone }).await;
                                        });
                                    }>
                                        <option value="Standard" selected=move || config.get().shield_level == "Standard">"Standard (Aggressive Ad & Tracker Filter)"</option>
                                        <option value="Aggressive" selected=move || config.get().shield_level == "Aggressive">"Aggressive (Strict Anti-Tracking & Fingerprinting)"</option>
                                        <option value="Off" selected=move || config.get().shield_level == "Off">"Off (No Shield)"</option>
                                    </select>
                                </div>
                            </div>
                        }.into_view(),

                        SettingsTab::Dns => view! {
                            <div class="panel-card">
                                <h2>"Secure DNS (DNS-over-HTTPS RFC 8484)"</h2>
                                <div class="grid-form">
                                    <label>"DoH Provider Endpoint"</label>
                                    <input type="text" prop:value=doh_input on:input=move |ev| set_doh_input.set(event_target_value(&ev)) />
                                    <div style="display:flex; gap:10px; align-items:center; margin-top:8px;">
                                        <button class="btn-action" on:click=test_dns>"Test Latency"</button>
                                        <span style="font-size:12px; color:var(--text-secondary);">{doh_result}</span>
                                    </div>
                                </div>
                            </div>
                        }.into_view(),

                        SettingsTab::Appearance => view! {
                            <div class="panel-card">
                                <h2>"Appearance & Themes"</h2>
                                <div class="grid-form">
                                    <label>"Theme Mode"</label>
                                    <select on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        let is_dark = val == "dark";
                                        let mut c = config.get();
                                        c.dark_theme = is_dark;
                                        set_config.set(c);
                                        spawn_local(async move {
                                            let _ = call_tauri::<_, ()>("update_setting", &SettingArgs { key: "dark_theme".into(), value: if is_dark { "true".into() } else { "false".into() } }).await;
                                        });
                                    }>
                                        <option value="dark" selected=move || config.get().dark_theme>"Dark (Titanium / Charcoal)"</option>
                                        <option value="light" selected=move || !config.get().dark_theme>"Light"</option>
                                    </select>
                                </div>
                            </div>
                        }.into_view(),

                        SettingsTab::About => view! {
                            <div class="panel-card">
                                <h2>"About Caram Browser"</h2>
                                <p style="font-size:13px; line-height:1.6; color:var(--text-secondary);">
                                    "Caram Browser Version 1.0.0 (Linux x86_64)"<br />
                                    "Engineered with Tauri v2, WebKitGTK 4.1, and 100% Rust WASM."<br />
                                    "Powered by Brave Adblock Rust Core, Argon2id & AES-256-GCM Vault."
                                </p>
                            </div>
                        }.into_view(),
                    }}
                </main>
            </div>
        </div>
    }
}
