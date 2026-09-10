use crate::tauri_ipc::call_tauri;
use leptos::*;
use serde::Serialize;
use shared::DownloadRecord;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct OpenPathArgs {
    path: String,
}

#[component]
pub fn DownloadsView() -> impl IntoView {
    let (downloads, set_downloads) = create_signal(Vec::<DownloadRecord>::new());

    let load_downloads = move || {
        spawn_local(async move {
            if let Ok(res) = call_tauri::<_, Vec<DownloadRecord>>("fetch_downloads", &EmptyArgs {}).await {
                set_downloads.set(res);
            }
        });
    };
    load_downloads();

    let clear_all = move |_| {
        spawn_local(async move {
            let _ = call_tauri::<_, ()>("clear_downloads", &EmptyArgs {}).await;
            set_downloads.set(Vec::new());
        });
    };

    view! {
        <div class="internal-view">
            <div class="panel-card">
                <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:15px;">
                    <h2>"Download Manager"</h2>
                    <button class="btn-action" style="background:var(--danger)" on:click=clear_all>"Clear Downloads"</button>
                </div>

                <table class="data-table">
                    <thead><tr><th>"Filename"</th><th>"Size"</th><th>"Status"</th><th>"Action"</th></tr></thead>
                    <tbody>
                        {move || downloads.get().into_iter().map(|item| {
                            let path = item.file_path.clone();
                            view! {
                                <tr>
                                    <td><strong>{item.filename}</strong></td>
                                    <td>{item.file_size}</td>
                                    <td><span style="color:var(--accent-shield)">{item.status}</span></td>
                                    <td>
                                        <button class="btn-action" style="padding:4px 10px; font-size:11px;" on:click=move |_| {
                                            let p = path.clone();
                                            spawn_local(async move {
                                                let _ = call_tauri::<_, ()>("open_file_manager", &OpenPathArgs { path: p }).await;
                                            });
                                        }>"Show in Folder"</button>
                                    </td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
