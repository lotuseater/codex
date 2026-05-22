#[path = "suite/review_history.rs"]
mod review_history;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_history_surfaces_in_parent_session() {
    review_history::review_history_surfaces_in_parent_session().await;
}
