#![cfg(not(target_os = "windows"))]

#[path = "view_image_common.rs"]
mod common;

use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_turn_with_local_image_attaches_image() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    assert_user_turn_local_image_resizes_to((2304, 864), (2048, 768)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_turn_with_vertical_local_image_resizes_to_square_bounds() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    assert_user_turn_local_image_resizes_to((1024, 4096), (512, 2048)).await
}
