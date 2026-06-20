use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn turn_start_defaults_local_image_detail_to_high() -> Result<()> {
    let input_images = run_local_image_turn(/*detail*/ None).await?;

    assert_eq!(input_images.len(), 1);
    assert_eq!(
        input_images[0].get("detail").and_then(Value::as_str),
        Some("high")
    );

    Ok(())
}

#[tokio::test]
async fn turn_start_forwards_custom_local_image_detail() -> Result<()> {
    let input_images = run_local_image_turn(Some(ImageDetail::Original)).await?;

    assert_eq!(input_images.len(), 1);
    assert_eq!(
        input_images[0].get("detail").and_then(Value::as_str),
        Some("original")
    );

    Ok(())
}
