use crate::tauri_ipc::call_tauri;
use leptos::*;
use serde::Serialize;
use shared::DecryptedVaultRecord;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct MasterPassArgs {
    master_pass: String,
}

#[derive(Serialize)]
struct AddVaultArgs {
    master_pass: String,
    website: String,
    username: String,
    secret: String,
}

#[derive(Serialize)]
struct DeleteVaultArgs {
    id: i64,
}

#[derive(Serialize)]
struct GenPassArgs {
    length: usize,
}

#[component]
pub fn VaultView() -> impl IntoView {
    let (configured, set_configured) = create_signal(false);
    let (unlocked_pass, set_unlocked_pass) = create_signal(Option::<String>::None);
    let (creds, set_creds) = create_signal(Vec::<DecryptedVaultRecord>::new());

    let (master_input, set_master_input) = create_signal(String::new());
    let (site_input, set_site_input) = create_signal(String::new());
    let (user_input, set_user_input) = create_signal(String::new());
    let (sec_input, set_sec_input) = create_signal(String::new());

    spawn_local(async move {
        if let Ok(c) = call_tauri::<_, bool>("vault_is_configured", &EmptyArgs {}).await {
            set_configured.set(c);
        }
    });

    let generate_random = move |_| {
        spawn_local(async move {
            if let Ok(pw) = call_tauri::<_, String>("generate_password", &GenPassArgs { length: 20 }).await {
                set_sec_input.set(pw);
            }
        });
    };

    let unlock = move |_| {
        let p = master_input.get();
        let p_clone = p.clone();
        spawn_local(async move {
            let res: Result<Vec<DecryptedVaultRecord>, _> = call_tauri("vault_read_all", &MasterPassArgs { master_pass: p.clone() }).await;
            if let Ok(list) = res {
                set_creds.set(list);
                set_unlocked_pass.set(Some(p_clone));
            }
                Err(_) => {}
            }
        });
    };

    let save_cred = move |_| {
        if let Some(pass) = unlocked_pass.get() {
            let site = site_input.get();
            let user = user_input.get();
            let sec = sec_input.get();
            let pass_clone = pass.clone();
            spawn_local(async move {
                let _ = call_tauri::<_, ()>("vault_save_credential", &AddVaultArgs {
                    master_pass: pass_clone.clone(),
                    website: site,
                    username: user,
                    secret: sec,
                }).await;
                if let Ok(list) = call_tauri::<_, Vec<DecryptedVaultRecord>>("vault_read_all", &MasterPassArgs { master_pass: pass_clone }).await {
                    set_creds.set(list);
                }
            });
        }
    };

    view! {
        <div class="internal-view">
            {move || if !configured.get() {
                view! {
                    <div class="panel-card" style="max-width:480px;">
                        <h2>"Setup Caram Vault"</h2>
                        <div class="grid-form">
                            <label>"Set Master Password (min 8 chars)"</label>
                            <input type="password" on:input=move |ev| set_master_input.set(event_target_value(&ev)) />
                            <button class="btn-action" on:click=move |_| {
                                let p = master_input.get();
                                spawn_local(async move {
                                    if call_tauri::<_, ()>("vault_setup", &MasterPassArgs { master_pass: p }).await.is_ok() {
                                        set_configured.set(true);
                                    }
                                });
                            }>"Initialize Vault"</button>
                        </div>
                    </div>
                }
            } else if unlocked_pass.get().is_none() {
                view! {
                    <div class="panel-card" style="max-width:440px;">
                        <h2>"Unlock Vault"</h2>
                        <div class="grid-form">
                            <label>"Master Password"</label>
                            <input type="password" on:input=move |ev| set_master_input.set(event_target_value(&ev)) />
                            <button class="btn-action" on:click=unlock>"Unlock"</button>
                        </div>
                    </div>
                }
            } else {
                view! {
                    <div class="panel-card">
                        <h2>"Password Vault (Argon2id + AES-256-GCM)"</h2>
                        <div class="grid-form" style="margin-bottom:20px; background:var(--bg-primary); padding:16px; border-radius:6px;">
                            <input type="text" placeholder="Website" on:input=move |ev| set_site_input.set(event_target_value(&ev)) />
                            <input type="text" placeholder="Username" on:input=move |ev| set_user_input.set(event_target_value(&ev)) />
                            <div style="display:flex; gap:10px;">
                                <input type="text" placeholder="Password" prop:value=sec_input on:input=move |ev| set_sec_input.set(event_target_value(&ev)) style="flex:1;" />
                                <button class="btn-action" style="background:var(--bg-tertiary)" on:click=generate_random>"Generate Strong"</button>
                            </div>
                            <button class="btn-action" on:click=save_cred>"Add Credential"</button>
                        </div>

                        <table class="data-table">
                            <thead><tr><th>"Website"</th><th>"Username"</th><th>"Secret"</th><th>"Date"</th><th>"Action"</th></tr></thead>
                            <tbody>
                                {move || creds.get().into_iter().map(|c| {
                                    let id = c.id;
                                    view! {
                                        <tr>
                                            <td>{c.website}</td>
                                            <td>{c.username}</td>
                                            <td><code>{c.secret}</code></td>
                                            <td>{c.created_at}</td>
                                            <td><button class="icon-btn" style="color:var(--danger)" on:click=move |_| {
                                                let p = unlocked_pass.get().unwrap_or_default();
                                                spawn_local(async move {
                                                    let _ = call_tauri::<_, ()>("vault_delete", &DeleteVaultArgs { id }).await;
                                                    if let Ok(list) = call_tauri::<_, Vec<DecryptedVaultRecord>>("vault_read_all", &MasterPassArgs { master_pass: p }).await {
                                                        set_creds.set(list);
                                                    }
                                                });
                                            }>"Delete"</button></td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }
            }}
        </div>
    }
}
