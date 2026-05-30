use super::super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn flush_answer_stream_keeps_default_reflow_for_plain_text_tail() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let cwd = chat.config.cwd.to_path_buf();

    let mut controller = crate::streaming::controller::StreamController::new(
        Some(80),
        cwd.as_path(),
        HistoryRenderMode::Rich,
    );
    assert!(controller.push("plain response line\n"));
    chat.stream_controller = Some(controller);

    while rx.try_recv().is_ok() {}

    chat.flush_answer_stream_with_separator();

    let mut saw_consolidate = false;
    let mut saw_insert_history = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            AppEvent::InsertHistoryCell(_) => saw_insert_history = true,
            AppEvent::ConsolidateAgentMessage {
                scrollback_reflow,
                deferred_history_cell,
                ..
            } => {
                saw_consolidate = true;
                assert_eq!(
                    scrollback_reflow,
                    crate::app_event::ConsolidationScrollbackReflow::IfResizeReflowRan
                );
                assert!(deferred_history_cell.is_none());
            }
            _ => {}
        }
    }

    assert!(
        saw_consolidate,
        "expected stream finalization to consolidate"
    );
    assert!(
        saw_insert_history,
        "plain text should still insert history before consolidation"
    );
}

#[tokio::test]
async fn flush_answer_stream_requests_scrollback_reflow_for_live_table_tail() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let cwd = chat.config.cwd.to_path_buf();

    let mut controller = crate::streaming::controller::StreamController::new(
        Some(80),
        cwd.as_path(),
        HistoryRenderMode::Rich,
    );
    controller.push("| Name | Notes |\n");
    controller.push("| --- | --- |\n");
    controller.push("| alpha | tail held until final table render |\n");
    assert!(
        controller.has_live_tail(),
        "expected table holdback to leave a live tail for this regression",
    );
    chat.stream_controller = Some(controller);

    while rx.try_recv().is_ok() {}

    chat.flush_answer_stream_with_separator();

    let mut saw_consolidate = false;
    let mut saw_insert_history = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            AppEvent::InsertHistoryCell(_) => saw_insert_history = true,
            AppEvent::ConsolidateAgentMessage {
                scrollback_reflow,
                deferred_history_cell,
                ..
            } => {
                saw_consolidate = true;
                assert_eq!(
                    scrollback_reflow,
                    crate::app_event::ConsolidationScrollbackReflow::Required
                );
                assert!(
                    deferred_history_cell.is_some(),
                    "live table tail should be staged for consolidation",
                );
            }
            _ => {}
        }
    }

    assert!(
        saw_consolidate,
        "expected stream finalization to consolidate"
    );
    assert!(
        !saw_insert_history,
        "live table tail should not be inserted before canonical reflow"
    );
}

#[tokio::test]
async fn completed_plan_table_tail_skips_provisional_history_insert() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let cwd = chat.config.cwd.to_path_buf();

    let mut controller = crate::streaming::controller::PlanStreamController::new(
        Some(80),
        cwd.as_path(),
        HistoryRenderMode::Rich,
    );
    controller.push("| Step | Owner |\n");
    controller.push("| --- | --- |\n");
    controller.push("| Verify | Codex |\n");
    assert!(
        controller.has_live_tail(),
        "expected plan table holdback to leave a live tail",
    );
    chat.plan_stream_controller = Some(controller);
    chat.transcript.plan_delta_buffer =
        "| Step | Owner |\n| --- | --- |\n| Verify | Codex |\n".to_string();

    while rx.try_recv().is_ok() {}

    chat.on_plan_item_completed(String::new());

    let mut saw_source_backed_plan = false;
    let mut saw_stream_plan = false;
    let mut rendered_plan = String::new();
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            if cell.as_any().is::<history_cell::ProposedPlanCell>() {
                saw_source_backed_plan = true;
                rendered_plan = lines_to_single_string(&cell.display_lines(/*width*/ 80));
            }
            saw_stream_plan |= cell.as_any().is::<history_cell::ProposedPlanStreamCell>();
        }
    }

    assert!(saw_source_backed_plan, "expected source-backed plan insert");
    assert!(
        rendered_plan.contains('│') || rendered_plan.contains('┌'),
        "expected completed plan table to render as a boxed table, got: {rendered_plan:?}"
    );
    assert!(
        !saw_stream_plan,
        "live plan table tail should not be inserted provisionally"
    );
}
