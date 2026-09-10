use crate::tauri_ipc::call_tauri;
use leptos::*;
use serde::Serialize;
use shared::HistoryRecord;

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn HistoryView() -> impl IntoView {
    let (history, set_history) = create_signal(Vec::<HistoryRecord>::new());

    let load_hist = move || {
        spawn_local(async move {
            if let Ok(res) = call_tauri::<_, Vec<HistoryRecord>>("fetch_history", &EmptyArgs {}).await {
                set_history.set(res);
            }
        });
    };
    load_hist();

    let clear_hist = move |_| {
        spawn_local(async move {
            let _ = call_tauri::<_, ()>("clear_history", &EmptyArgs {}).await;
            set_history.set(Vec::new());
        });
    };

    view! {
        <div class="internal-view">
            <div class="panel-card">
                <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:15px;">
                    <h2>"Browsing History"</h2>
                    <button class="btn-action" style="background:var(--danger)" on:click=clear_hist>"Clear History"</button>
                </div>
                <table class="data-table">
                    <thead><tr><th>"Title"</th><th>"URL"</th><th>"Date"</th></tr></thead>
                    <tbody>
                        {move || history.get().into_iter().map(|item| {
                            view! {
                                <tr>
                                    <td>{item.title}</td>
                                    <td><span style="color:var(--accent)">{item.url}</span></td>
                                    <td>{item.timestamp.unwrap_or_default()}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
