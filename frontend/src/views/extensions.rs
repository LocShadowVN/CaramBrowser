use crate::tauri_ipc::call_tauri;
use leptos::*;
use serde::Serialize;
use shared::ExtensionItem;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct ToggleExtArgs {
    id: String,
    enabled: bool,
}

#[derive(Serialize)]
struct RemoveExtArgs {
    id: String,
}

#[derive(Serialize)]
struct LoadUnpackedArgs {
    folder_path: String,
}

#[component]
pub fn ExtensionsView() -> impl IntoView {
    let (extensions, set_extensions) = create_signal(Vec::<ExtensionItem>::new());
    let (path_input, set_path_input) = create_signal(String::new());

    let load_exts = move || {
        spawn_local(async move {
            if let Ok(res) = call_tauri::<_, Vec<ExtensionItem>>("fetch_extensions", &EmptyArgs {}).await {
                set_extensions.set(res);
            }
        });
    };
    load_exts();

    let load_unpacked = move |_| {
        let p = path_input.get();
        spawn_local(async move {
            let _ = call_tauri::<_, ExtensionItem>("load_unpacked_extension", &LoadUnpackedArgs { folder_path: p }).await;
            if let Ok(res) = call_tauri::<_, Vec<ExtensionItem>>("fetch_extensions", &EmptyArgs {}).await {
                set_extensions.set(res);
            }
        });
    };

    view! {
        <div class="internal-view">
            <div class="panel-card">
                <h2>"Extension Architecture"</h2>
                <p style="font-size:12px; color:var(--text-secondary); margin-bottom:15px;">
                    "Developer Mode: Load custom extensions directly from folder using manifest.json"
                </p>

                <div style="display:flex; gap:10px; margin-bottom:20px;">
                    <input type="text" placeholder="/path/to/extension_folder" prop:value=path_input on:input=move |ev| set_path_input.set(event_target_value(&ev)) style="flex:1;" />
                    <button class="btn-action" on:click=load_unpacked>"Load Unpacked"</button>
                </div>

                <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(280px, 1fr)); gap:16px;">
                    {move || extensions.get().into_iter().map(|item| {
                        let id = item.id.clone();
                        let id_del = item.id.clone();
                        let enabled = item.enabled;
                        view! {
                            <div style="background:var(--bg-primary); border:1px solid var(--border); border-radius:8px; padding:16px; display:flex; flex-direction:column; justify-content:space-between;">
                                <div>
                                    <div style="display:flex; justify-content:space-between; align-items:center;">
                                        <strong>{item.name}</strong>
                                        <span style="font-size:11px; color:var(--text-secondary)">{item.version}</span>
                                    </div>
                                    <p style="font-size:12px; color:var(--text-secondary); margin-top:8px;">{item.description}</p>
                                </div>
                                <div style="display:flex; justify-content:space-between; align-items:center; margin-top:16px;">
                                    <button class="btn-action" style=format!("background:{}", if enabled { "var(--accent-shield)" } else { "var(--border)" }) on:click=move |_| {
                                        let id_c = id.clone();
                                        spawn_local(async move {
                                            let _ = call_tauri::<_, ()>("toggle_extension", &ToggleExtArgs { id: id_c, enabled: !enabled }).await;
                                            if let Ok(res) = call_tauri::<_, Vec<ExtensionItem>>("fetch_extensions", &EmptyArgs {}).await {
                                                set_extensions.set(res);
                                            }
                                        });
                                    }>{if enabled { "Enabled" } else { "Disabled" }}</button>

                                    <button class="icon-btn" style="color:var(--danger)" on:click=move |_| {
                                        let id_c = id_del.clone();
                                        spawn_local(async move {
                                            let _ = call_tauri::<_, ()>("remove_extension", &RemoveExtArgs { id: id_c }).await;
                                            if let Ok(res) = call_tauri::<_, Vec<ExtensionItem>>("fetch_extensions", &EmptyArgs {}).await {
                                                set_extensions.set(res);
                                            }
                                        });
                                    }>"Remove"</button>
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}
