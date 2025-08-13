use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

const BACKEND_URL: &str = "/api";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Review {
    review_title: String,
    review_body: String,
    product_id: String,
    review_rating: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SearchQuery {
    query: String,
    top_k: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    score: f32,
    review: Review,
}

#[component]
fn InsertForm() -> impl IntoView {
    let (title, set_title) = create_signal(String::new());
    let (body, set_body) = create_signal(String::new());
    let (product_id, set_product_id) = create_signal(String::new());
    let (rating, set_rating) = create_signal(5_i32);
    let (status, set_status) = create_signal(String::new());

    let on_submit = move |_| {
        let review = Review {
            review_title: title.get().to_string(),
            review_body: body.get().to_string(),
            product_id: product_id.get().to_string(),
            review_rating: rating.get(),
        };
        set_status.set("Sending...".into());
        spawn_local(async move {
            let resp = Request::post(&format!("{}/reviews", BACKEND_URL))
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&review).unwrap()).unwrap()
                .send()
                .await;
            match resp {
                Ok(r) if r.status() == 201 => set_status.set("✓ Saved".into()),
                Ok(r) => set_status.set(format!("Error: {}", r.status()).into()),
                Err(e) => set_status.set(format!("Network error: {e}").into()),
            }
        });
    };

    view! {
        <h2>"Insert Review"</h2>
        <div>
            <input placeholder="Title" prop:value=title on:input=move |e| set_title.set(event_target_value(&e)) />
            <textarea placeholder="Body" prop:value=body on:input=move |e| set_body.set(event_target_value(&e))/>
            <input placeholder="Product ID" prop:value=product_id on:input=move |e| set_product_id.set(event_target_value(&e)) />
            <input type="number" min="1" max="5" prop:value=rating on:input=move |e| set_rating.set(event_target_value(&e).parse().unwrap_or(5)) />
            <button on:click=on_submit>"Submit"</button>
            <p>{ status }</p>
        </div>
    }
}

#[component]
fn SearchPage() -> impl IntoView {
    let (query, set_query) = create_signal(String::new());
    let (topk, set_topk) = create_signal(5_usize);
    let (results, set_results) = create_signal(Vec::<SearchResult>::new());
    let do_search = move |_| {
        let q = query.get();
        let k = topk.get();
        spawn_local(async move {
            let payload = SearchQuery { query: q, top_k: Some(k) };
            match Request::post(&format!("{}/search", BACKEND_URL))
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&payload).unwrap()).unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.status() == 200 => {
                    if let Ok(data) = resp.json::<Vec<SearchResult>>().await {
                        set_results.set(data);
                    }
                }
                _ => log::error!("search failed"),
            }
        });
    };

    view! {
        <h2>"Semantic Search"</h2>
        <input placeholder="Query" prop:value=query on:input=move |e| set_query.set(event_target_value(&e)) />
        <input type="number" min="1" prop:value=topk on:input=move |e| set_topk.set(event_target_value(&e).parse().unwrap_or(5)) />
        <button on:click=do_search>"Search"</button>
        <ul>
            <For each=move || results.get() key=|r| r.review.review_title.clone() let:res>
                <li>
                    <p><b>{ res.review.review_title.clone() }</b> " (" {res.score} ")"</p>
                    <p>{ res.review.review_body.clone() }</p>
                </li>
            </For>
        </ul>
    }
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <nav>
                <A href="/">"Insert"</A>
                " | "
                <A href="/search">"Search"</A>
            </nav>
            <Routes>
                <Route path="/" view=InsertForm />
                <Route path="/search" view=SearchPage />
            </Routes>
        </Router>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_log::init_with_level(log::Level::Debug).expect("log");
    mount_to_body(|| view! { <App/> });
}
