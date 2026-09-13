use crate::tauri_ipc::call_tauri;
use leptos::*;
use serde::Serialize;
use shared::BookmarkRecord;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct DeleteBmArgs {
    id: i64,
}

#[component]
pub fn BookmarksView<F>(on_navigate: F) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{
    let (bookmarks, set_bookmarks) = create_signal(Vec::<BookmarkRecord>::new());
    let (filter, set_filter) = create_signal(String::new());

    let load_bm = move || {
        spawn_local(async move {
            if let Ok(res) = call_tauri::<_, Vec<BookmarkRecord>>("fetch_bookmarks", &EmptyArgs {}).await {
                set_bookmarks.set(res);
            }
        });
    };
    load_bm();

    view! {
        <div class="internal-view">
            <div class="panel-card">
                <h2>"Bookmark Manager"</h2>
                <input
                    type="text"
                    placeholder="Search bookmarks..."
                    prop:value=filter
                    on:input=move |ev| set_filter.set(event_target_value(&ev))
                    style="width: 100%; margin-bottom: 15px;"
                />

                <table class="data-table">
                    <thead><tr><th>"Title"</th><th>"URL"</th><th>"Action"</th></tr></thead>
                    <tbody>
                        {move || {
                            let f = filter.get().to_lowercase();
                            bookmarks.get().into_iter()
                                .filter(|item| f.is_empty() || item.title.to_lowercase().contains(&f) || item.url.to_lowercase().contains(&f))
                                .map(|item| {
                                    let u = item.url.clone();
                                    let id = item.id.unwrap_or(0);
                                    view! {
                                        <tr>
                                            <td><strong>{item.title}</strong></td>
                                            <td><span style="color:var(--accent); cursor:pointer;" on:click=move |_| on_navigate(u.clone())>{item.url}</span></td>
                                            <td><button class="icon-btn" style="color:var(--danger)" on:click=move |_| {
                                                spawn_local(async move {
                                                    let _ = call_tauri::<_, ()>("remove_bookmark", &DeleteBmArgs { id }).await;
                                                    if let Ok(res) = call_tauri::<_, Vec<BookmarkRecord>>("fetch_bookmarks", &EmptyArgs {}).await {
                                                        set_bookmarks.set(res);
                                                    }
                                                });
                                            }>"Delete"</button></td>
                                        </tr>
                                    }
                                }).collect_view()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
