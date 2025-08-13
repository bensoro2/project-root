use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;

use crate::models::{Review, SearchQuery, SearchResult};

const BACKEND_URL: &str = "/api";

#[component]
fn InsertForm() -> impl IntoView {
    let (title, set_title) = create_signal(String::new());
    let (body, set_body) = create_signal(String::new());
    let (product_id, set_product_id) = create_signal(String::new());
    let (rating, set_rating) = create_signal(5_i32);
    let (status, set_status) = create_signal(String::new());
    let (is_loading, set_loading) = create_signal(false);

    let is_valid = move || {
        !title.get().trim().is_empty() &&
        !body.get().trim().is_empty() &&
        !product_id.get().trim().is_empty() &&
        (1..=5).contains(&rating.get())
    };

    let on_submit = move |_| {
        if !is_valid() {
            set_status.set("Please fill in all fields correctly".into());
            return;
        }

        let review = Review {
            review_title: title.get().trim().to_string(),
            review_body: body.get().trim().to_string(),
            product_id: product_id.get().trim().to_string(),
            review_rating: rating.get(),
        };

        set_status.set("Sending...".into());
        set_loading.set(true);

        spawn_local(async move {
            let request = match Request::post(&format!("{}/reviews", BACKEND_URL))
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&review).unwrap()) {
                    Ok(req) => req,
                    Err(e) => {
                        set_loading.set(false);
                        set_status.set(format!("Request creation error: {}", e).into());
                        return;
                    }
                };

            let result = request.send().await;

            set_loading.set(false);
            
            match result {
                Ok(response) => {
                    match response.status() {
                        201 => {
                            set_status.set("✓ Review saved successfully!".into());
                            set_title.set(String::new());
                            set_body.set(String::new());
                            set_product_id.set(String::new());
                            set_rating.set(5);
                        }
                        400 => set_status.set("Error: Invalid review data".into()),
                        500 => set_status.set("Error: Server error".into()),
                        status => set_status.set(format!("Error: HTTP {}", status).into()),
                    }
                }
                Err(error) => {
                    set_status.set(format!("Network error: {}", error).into());
                }
            }
        });
    };

    view! {
        <h2>"Insert Review"</h2>
        <div class="card">
            <form on:submit=move |e| { e.prevent_default(); on_submit(e); }>
                <div class="form-group">
                    <div class="form-control" class:empty=move || title.get().trim().is_empty()>
                        <input
                            id="title"
                            type="text"
                            placeholder=" "
                            prop:value=title
                            on:input=move |e| set_title.set(event_target_value(&e))
                            required
                            disabled=is_loading
                        />
                        <label for="title">"Review Title"</label>
                    </div>
                </div>
                <div class="form-group">
                    <div class="form-control" class:empty=move || body.get().trim().is_empty()>
                        <textarea
                            id="body"
                            placeholder=" "
                            prop:value=body
                            on:input=move |e| set_body.set(event_target_value(&e))
                            required
                            disabled=is_loading
                        ></textarea>
                        <label for="body">"Review Body"</label>
                    </div>
                </div>
                <div class="form-group">
                    <div class="form-control" class:empty=move || product_id.get().trim().is_empty()>
                        <input
                            id="product-id"
                            type="text"
                            placeholder=" "
                            prop:value=product_id
                            on:input=move |e| set_product_id.set(event_target_value(&e))
                            required
                            disabled=is_loading
                        />
                        <label for="product-id">"Product ID"</label>
                    </div>
                </div>
                <div class="form-group">
                    <div class="form-control">
                        <input
                            id="rating"
                            type="number"
                            min="1"
                            max="5"
                            prop:value=rating
                            on:input=move |e| {
                                let value = event_target_value(&e).parse().unwrap_or(5);
                                set_rating.set(value.clamp(1, 5));
                            }
                            disabled=is_loading
                        />
                        <label for="rating">"Rating"</label>
                    </div>
                </div>
                <div>
                    <button
                        type="submit"
                        disabled=move || is_loading.get() || !is_valid()
                    >
                        {move || if is_loading.get() { "Submitting..." } else { "Submit Review" }}
                    </button>
                </div>
                <div class="status">
                    <p>{ status }</p>
                </div>
            </form>
        </div>
    }
}

#[component]
fn SearchPage() -> impl IntoView {
    let (query, set_query) = create_signal(String::new());
    let (topk, set_topk) = create_signal(5_usize);
    let (results, set_results) = create_signal(Vec::<SearchResult>::new());
    let (is_loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(String::new());

    let is_valid = move || !query.get().trim().is_empty() && topk.get() > 0;

    let do_search = move |_| {
        if !is_valid() {
            set_error.set("Please enter a query and valid number of results".into());
            return;
        }

        let q = query.get();
        let k = topk.get();
        
        set_loading.set(true);
        set_error.set(String::new());
        set_results.set(Vec::new());

        spawn_local(async move {
            let payload = SearchQuery {
                query: q.trim().to_string(),
                top_k: Some(k)
            };
            
            let request = match Request::post(&format!("{}/search", BACKEND_URL))
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&payload).unwrap()) {
                    Ok(req) => req,
                    Err(e) => {
                        set_loading.set(false);
                        set_error.set(format!("Request creation error: {}", e).into());
                        return;
                    }
                };

            match request.send().await {
                Ok(response) => {
                    match response.status() {
                        200 => {
                            match response.json::<Vec<SearchResult>>().await {
                                Ok(data) => {
                                    set_results.set(data);
                                }
                                Err(e) => {
                                    set_error.set(format!("Failed to parse response: {}", e).into());
                                    log::error!("JSON parse error: {}", e);
                                }
                            }
                        }
                        400 => {
                            set_error.set("Error: Invalid search query".into());
                        }
                        500 => {
                            set_error.set("Error: Server error".into());
                        }
                        status => {
                            set_error.set(format!("Error: HTTP {}", status).into());
                        }
                    }
                }
                Err(e) => {
                    set_error.set(format!("Network error: {}", e).into());
                    log::error!("Request failed: {}", e);
                }
            }
            
            set_loading.set(false);
        });
    };

    view! {
        <h2>"Semantic Search"</h2>
        <div class="card">
            <form on:submit=move |e| { e.prevent_default(); do_search(e); }>
                <div class="form-group">
                    <div class="form-control" class:empty=move || query.get().trim().is_empty()>
                        <input
                            id="search-query"
                            type="text"
                            placeholder=" "
                            prop:value=query
                            on:input=move |e| {
                                set_query.set(event_target_value(&e));
                                set_error.set(String::new());
                            }
                            required
                            disabled=is_loading
                        />
                        <label for="search-query">"Search Query"</label>
                    </div>
                </div>
                <div class="form-group">
                    <div class="form-control">
                        <input
                            id="top-k"
                            type="number"
                            min="1"
                            max="20"
                            prop:value=topk
                            on:input=move |e| {
                                let value = event_target_value(&e).parse().unwrap_or(5);
                                set_topk.set(value.clamp(1, 20));
                            }
                            disabled=is_loading
                        />
                        <label for="top-k">"Number of Results"</label>
                    </div>
                </div>
                <div>
                    <button
                        type="submit"
                        disabled=move || is_loading.get() || !is_valid()
                    >
                        {move || if is_loading.get() { "Searching..." } else { "Search" }}
                    </button>
                </div>
            </form>
            
            {move || {
                let error_msg = error.get();
                if !error_msg.is_empty() {
                    view! {
                        <div class="error">
                            <p>{error_msg}</p>
                        </div>
                    }.into_view()
                } else {
                    ().into_view()
                }
            }}
            
            {move || {
                let results_list = results.get();
                if results_list.is_empty() && !is_loading.get() && query.get().is_empty() {
                    view! {
                        <div class="no-results">
                            <p>"Enter a query to search for similar reviews"</p>
                        </div>
                    }.into_view()
                } else if results_list.is_empty() && !is_loading.get() && !query.get().is_empty() {
                    view! {
                        <div class="no-results">
                            <p>"No matching reviews found"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <ul>
                            <For each=move || results.get() key=|r| r.review.review_title.clone() let:res>
                                <li>
                                    <div class="result-header">
                                        <span class="score">"Score: " {format!("{:.3}", res.score)}</span>
                                    </div>
                                    <h3>{ res.review.review_title.clone() }</h3>
                                    <p>{ res.review.review_body.clone() }</p>
                                    <div class="result-meta">
                                        <small>"Product ID: " {res.review.product_id.clone()}</small>
                                        <small>"Rating: " {res.review.review_rating} "/5"</small>
                                    </div>
                                </li>
                            </For>
                        </ul>
                    }.into_view()
                }
            }}
        </div>
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

mod models {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct Review {
        pub review_title: String,
        pub review_body: String,
        pub product_id: String,
        pub review_rating: i32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct SearchQuery {
        pub query: String,
        pub top_k: Option<usize>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SearchResult {
        pub score: f32,
        pub review: Review,
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    mount_to_body(|| view! { <App/> });
}
