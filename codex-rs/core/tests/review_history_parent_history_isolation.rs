#[path = "suite/review_history.rs"]
mod review_history;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_input_isolated_from_parent_history() {
    review_history::review_input_isolated_from_parent_history().await;
}
