use std::path::Path;

use sorter_core::*;

#[test]
fn exact_public_signatures_accept_only_journal_paths_and_bound_facts() {
    let _: fn(&Plan) -> Result<JournalBinding, EngineError> = JournalBinding::from_plan;
    let _: fn(&Path, &JournalBinding) -> Result<Journal, EngineError> = Journal::open;
    let _: fn(&mut Journal, &JournalEvent) -> Result<(), EngineError> = Journal::append;
    let _: fn(&Path, &JournalBinding) -> Result<ReplayState, EngineError> = replay;
}
