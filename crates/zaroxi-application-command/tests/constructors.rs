use zaroxi_application_command::ports::{CommandRecord, CommandKind, AppCommand};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_kernel_types::Id;

#[test]
fn command_record_constructors() {
    let sid = Id::new();
    let wid = Id::new();
    let bid = BufferId::from("buf:1");

    let rec = CommandRecord::new_success(
        CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: bid.clone() } },
        Some(sid),
        Some(wid),
        Some(bid.clone()),
        Some("ok".to_string()),
    );
    assert!(rec.success);
    assert_eq!(rec.result.unwrap(), "ok");
    assert!(rec.error.is_none());

    let rec2 = CommandRecord::new_failure(
        CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: bid.clone() } },
        Some(sid),
        None,
        Some(bid.clone()),
        Some("err".to_string()),
    );
    assert!(!rec2.success);
    assert_eq!(rec2.error.unwrap(), "err");
}
