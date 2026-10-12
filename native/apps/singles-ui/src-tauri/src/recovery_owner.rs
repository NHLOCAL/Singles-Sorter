//! Restart inspection accepts only opaque retained journal/root tokens. Journal
//! contents are data, never executable authority. No replay or cleanup exists.
use super::*;

#[derive(Clone, Debug, Serialize)]
pub struct RecoveryRecord {
    token: String,
    job_id: Option<String>,
    journal_name: String,
    family: &'static str,
    source: Option<String>,
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_parent: Option<String>,
    state: &'static str,
    detail: String,
    operation_count: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryPage {
    records: Vec<RecoveryRecord>,
    total: usize,
    offset: usize,
    truncated: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryOperation {
    operation: usize,
    source: String,
    destination: String,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    evidence: Vec<RecoveryEvidence>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryEvidence {
    role: String,
    path: String,
    status: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryReport {
    job_id: String,
    state: &'static str,
    observed_directories: usize,
    pending_directories: usize,
    rejected_directories: usize,
    incomplete_tail: bool,
    unverified_directories: usize,
    observation_limit_reached: bool,
    operations: Vec<RecoveryOperation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    evidence: Vec<RecoveryEvidence>,
}
#[path = "recovery_run.rs"]
mod run;
#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
use run::RecoveryOutput;
pub use run::{RecoveryProgress, RecoveryRunId, RecoverySnapshot};
#[derive(Default)]
pub(super) struct RecoveryState {
    pub(super) run: Option<run::RecoveryRun>,
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    records: Vec<selected::RetainedRecord>,
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    roots: Vec<Root>,
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    parents: Vec<selected::ParentRoot>,
}
#[cfg(not(all(windows, feature = "selected-windows-execution")))]
impl Host {
    pub fn start_recovery_list(&self, _: usize, _: usize, _: BusyGuard) -> Result<RecoveryRunId> {
        Err(HostError::new(
            "RECOVERY_UNAVAILABLE",
            "בדיקת יומני ביצוע אינה זמינה בבנייה זו.",
        ))
    }
    pub fn start_recovery_inspection(
        &self,
        _: String,
        _: String,
        _: Option<String>,
        _: Option<String>,
        _: BusyGuard,
    ) -> Result<RecoveryRunId> {
        Err(HostError::new(
            "RECOVERY_UNAVAILABLE",
            "בדיקת יומני ביצוע אינה זמינה בבנייה זו.",
        ))
    }
    pub fn select_recovery_parent_path(&self, _: &str, _: PathBuf) -> Result<RootSelection> {
        Err(HostError::new("RECOVERY_UNAVAILABLE", "בדיקת יומני ביצוע אינה זמינה בבנייה זו."))
    }
    pub fn select_recovery_path(&self, _: Role, _: PathBuf) -> Result<RootSelection> {
        Err(HostError::new(
            "RECOVERY_UNAVAILABLE",
            "בדיקת יומני ביצוע אינה זמינה בבנייה זו.",
        ))
    }
}
#[cfg(all(windows, feature = "selected-windows-execution"))]
mod selected {
    use super::*;
    use sorter_core::discovery::StorageErrorKind;
    use sorter_executor::inspection_control::InspectionControl;
    use sorter_executor::windows_album::{RecordedAlbumFileJob, inspect_recorded_album_file_job};
    use sorter_executor::windows_job::{
        MAX_RESTART_JOURNAL_BYTES, MAX_STREAMING_JOURNAL_BYTES, recover_selected_journals,
        recover_selected_journals_controlled, summarize_recorded_selected_job_with_control,
    };
    use sorter_executor::windows_repair::{
        RepairEvidenceStatus, RepairStatus, inspect_recorded_repair_job,
        recover_repair_journal_controlled,
    };
    use sorter_io::desktop::windows_mutation::{WindowsMutationStorage, WindowsSelectedRootGrant};
    use std::io::Read;

    const MAX_DIRECTORY_ENTRIES: usize = 4096;
    const MAX_RECORDS: usize = 256;
    const PAGE_LIMIT: usize = 10;
    const PAGE_BYTE_BUDGET: usize = 128 * 1024 * 1024;
    #[derive(Clone)]
    struct JournalEvidence {
        observed: EntryObservation,
        sha256: String,
    }
    #[derive(Clone)]
    pub(super) struct RetainedRecord {
        token: String,
        directory: PathBuf,
        root: EntryObservation,
        journal: RetainedJournal,
    }
    #[derive(Clone)]
    pub(super) struct ParentRoot {
        record_token: String,
        selection: RootSelection,
        path: PathBuf,
        observed: EntryObservation,
    }
    #[derive(Clone)]
    enum SelectedSource {
        Present(Root),
        Missing(ParentRoot),
    }
    #[derive(Clone)]
    enum RetainedJournal {
        Album {
            manifest: JournalEvidence,
            transfers: Option<JournalEvidence>,
            transfer_path: PathBuf,
            descriptor: RecordedAlbumFileJob,
        },
        Single {
            directories: JournalEvidence,
            transfers: Option<JournalEvidence>,
            transfer_path: PathBuf,
        },
        Repair {
            journal: JournalEvidence,
        },
    }
    fn recorded_error(error: sorter_executor::ExecutionError) -> HostError {
        if error == sorter_executor::ExecutionError::Cancelled {
            HostError::new("CANCELLED", "הבדיקה בוטלה. היומנים נשמרו.")
        } else if error == sorter_executor::ExecutionError::Unsupported {
            HostError::new(
                "RECOVERY_LIMIT",
                "הפעולה חורגת ממגבלת בדיקת הפתיחה מחדש. יש לשמור את היומן ולבדוק את הקבצים לפני ביצוע נוסף.",
            )
        } else {
            storage_error(error)
        }
    }
    fn changed() -> HostError {
        HostError::new(
            "JOURNAL_CHANGED",
            "היומן או התיקיות השתנו. יש לרענן את רשימת הפעולות ולבחור מחדש.",
        )
    }
    fn same_file(a: &EntryObservation, b: &EntryObservation) -> bool {
        a.reference == b.reference
            && a.identity == b.identity
            && a.identity.is_some()
            && a.kind == EntryKind::File
            && b.kind == EntryKind::File
            && a.size == b.size
            && a.modified == b.modified
    }
    fn checkpoint(control: Option<&InspectionControl>) -> Result<()> {
        control.map_or(Ok(()), |control| {
            control.checkpoint().map_err(recorded_error)
        })
    }
    fn probe<T>(
        control: Option<&InspectionControl>,
        action: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        checkpoint(control)?;
        if let Some(control) = control {
            control.observation().map_err(recorded_error)?;
        }
        let result = action();
        checkpoint(control)?;
        result
    }
    fn read_bound(
        storage: &mut Desktop,
        observed: &EntryObservation,
        budget: &mut usize,
        control: Option<&InspectionControl>,
    ) -> Result<(Vec<u8>, JournalEvidence)> {
        checkpoint(control)?;
        let size = observed.size.ok_or_else(changed)?;
        let max = if control.is_some() {
            MAX_STREAMING_JOURNAL_BYTES
        } else {
            MAX_RESTART_JOURNAL_BYTES
        };
        if observed.kind != EntryKind::File || size > max as u64 || size > *budget as u64 {
            return Err(HostError::new(
                "JOURNAL_LIMIT",
                "היומן גדול ממגבלת הבדיקה הבטוחה. הקובץ נשמר ללא שינוי.",
            ));
        }
        let mut input = probe(control, || {
            storage
                .open_read(&observed.reference)
                .map_err(storage_error)
        })?;
        let binding = input.binding().ok_or_else(changed)?;
        if Some(&binding.identity) != observed.identity.as_ref()
            || binding.size != size
            || Some(&binding.modified) != observed.modified.as_ref()
        {
            return Err(changed());
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size as usize)
            .map_err(storage_error)?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65_536];
        loop {
            let amount = (size as usize - bytes.len()).min(buffer.len());
            if amount == 0 {
                break;
            }
            checkpoint(control)?;
            if let Some(control) = control {
                control.observation().map_err(recorded_error)?;
            }
            let read = input.read(&mut buffer[..amount]);
            if let (Some(control), Ok(count)) = (control, &read) {
                control.bytes(*count).map_err(recorded_error)?;
            }
            checkpoint(control)?;
            let count = read.map_err(storage_error)?;
            if count == 0 {
                return Err(changed());
            }
            *budget = budget.checked_sub(count).ok_or_else(changed)?;
            hash.update(&buffer[..count]);
            bytes.extend_from_slice(&buffer[..count]);
        }
        checkpoint(control)?;
        if let Some(control) = control {
            control.observation().map_err(recorded_error)?;
        }
        let mut extra = [0u8; 1];
        let read = input.read(&mut extra);
        if let (Some(control), Ok(count)) = (control, &read) {
            control.bytes(*count).map_err(recorded_error)?;
        }
        checkpoint(control)?;
        let count = read.map_err(storage_error)?;
        *budget = budget.checked_sub(count).ok_or_else(changed)?;
        if count != 0
            || !same_file(
                observed,
                &probe(control, || {
                    storage.stat(&observed.reference).map_err(storage_error)
                })?,
            )
        {
            return Err(changed());
        }
        let evidence = JournalEvidence {
            observed: observed.clone(),
            sha256: format!("{:x}", hash.finalize()),
        };
        Ok((bytes, evidence))
    }
    fn observation(
        storage: &mut Desktop,
        path: &Path,
        control: Option<&InspectionControl>,
    ) -> Result<Option<EntryObservation>> {
        probe(control, || {
            let reference = match storage.reference(path) {
                Ok(reference) => reference,
                Err(error) if error.kind == StorageErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(storage_error(error)),
            };
            checkpoint(control)?;
            match storage.stat(&reference) {
                Ok(observed) => Ok(Some(observed)),
                Err(error) if error.kind == StorageErrorKind::NotFound => Ok(None),
                Err(error) => Err(storage_error(error)),
            }
        })
    }
    fn read_retained(
        storage: &mut Desktop,
        evidence: &JournalEvidence,
        budget: &mut usize,
        control: Option<&InspectionControl>,
    ) -> Result<Vec<u8>> {
        let current = probe(control, || {
            storage
                .stat(&evidence.observed.reference)
                .map_err(storage_error)
        })?;
        if !same_file(&evidence.observed, &current) {
            return Err(changed());
        }
        let (bytes, checked) = read_bound(storage, &current, budget, control)?;
        if evidence.sha256 != checked.sha256 {
            return Err(changed());
        }
        Ok(bytes)
    }
    fn control_for(cancel: &Cancellation, host: &Host, id: &str) -> InspectionControl<'static> {
        let host = host.clone();
        let id = id.to_owned();
        InspectionControl::new(cancel.cancelled.clone(), move |progress| {
            host.update_recovery_progress(&id, progress.into())
        })
    }
    fn single_record_detail(recorded_complete: bool) -> &'static str {
        if recorded_complete {
            "היומן מתעד סיום; מצב הקבצים הנוכחי דורש בדיקה."
        } else {
            "יש לבחור מחדש את תיקיות המקור והיעד כדי לבדוק את הקבצים. היומן לבדו אינו מאמת את מצבם."
        }
    }
    fn display(reference: &StorageRef) -> String {
        match reference {
            StorageRef::WindowsPath { utf16 } => String::from_utf16_lossy(utf16),
            _ => "מיקום שאינו נתמך".into(),
        }
    }
    fn journal_descriptor(name: &str) -> Option<(&str, &'static str)> {
        for (suffix, prefix, family) in [
            (".directories.jsonl", "sorter-", "single"),
            (".transfers.jsonl", "sorter-", "orphan"),
            (".album.jsonl", "album-", "album"),
            (".album-move.jsonl", "album-", "album"),
            (".transfers.jsonl", "album-", "orphan"),
            (".repair.jsonl", "repair-", "repair"),
        ] {
            if let Some(stem) = name.strip_suffix(suffix) {
                if stem.starts_with(prefix)
                    && stem.len() <= 100
                    && stem
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                {
                    return Some((stem, family));
                }
            }
        }
        None
    }
    fn journal_order(name: &str) -> (Option<u128>, &str) {
        // Ordering is display-only; a generated-looking name is not authority.
        let time = journal_descriptor(name)
            .and_then(|(stem, _)| stem.split('-').nth(2))
            .and_then(|time| time.parse().ok());
        (time, name)
    }
    impl Host {
        pub fn start_recovery_list(
            &self,
            offset: usize,
            limit: usize,
            guard: BusyGuard,
        ) -> Result<RecoveryRunId> {
            self.start_recovery_run("list", None, None, guard, move |cancel, host, id, _| {
                let control = control_for(cancel, host, id);
                let result = host
                    .list_recovery_with_control(offset, limit, Some(&control))
                    .map(RecoveryOutput::Page);
                host.update_recovery_progress(
                    id,
                    control.snapshot().map_err(recorded_error)?.into(),
                );
                result
            })
        }
        pub fn start_recovery_inspection(
            &self,
            token: String,
            source: String,
            target: Option<String>,
            resume: Option<String>,
            guard: BusyGuard,
        ) -> Result<RecoveryRunId> {
            let identity = Some((token.clone(), source.clone(), target.clone()));
            self.start_recovery_run(
                "inspect",
                identity,
                resume.as_deref(),
                guard,
                move |cancel, host, id, cursor| {
                    let control = control_for(cancel, host, id);
                    let result = host
                        .inspect_recovery_with_control(
                            &token,
                            &source,
                            target.as_deref(),
                            Some(&control),
                            cursor,
                        )
                        .map(RecoveryOutput::Report);
                    host.update_recovery_progress(
                        id,
                        control.snapshot().map_err(recorded_error)?.into(),
                    );
                    result
                },
            )
        }
        pub fn select_recovery_parent_path(&self, token: &str, path: PathBuf) -> Result<RootSelection> {
            let expected = {
                let inner = self.lock()?;
                let record = inner.recovery.records.iter().find(|record| record.token == token).ok_or_else(changed)?;
                let RetainedJournal::Album { descriptor, .. } = &record.journal else {
                    return Err(HostError::new("UNSUPPORTED_RECOVERY_PARENT", "לפעולה זו יש לבחור את תיקיית המקור עצמה."));
                };
                descriptor.original_source.as_ref().ok_or_else(|| HostError::new(
                    "UNSUPPORTED_RECOVERY_PARENT", "היומן אינו כולל ראיה למיקום המקור שהוסר. יש לבחור מקור קיים."
                ))?.parent_identity.clone()
            };
            validate_path(&path)?;
            let observed = inspect_root(&path)?;
            if observed.identity.as_ref() != Some(&expected) {
                return Err(HostError::new("RECOVERY_PARENT_CHANGED", "התיקייה שנבחרה אינה התיקייה שהכילה את המקור המתועד."));
            }
            // The identity supplied to native open is the fresh picker observation,
            // not the data-only expected identity read from the saved journal.
            sorter_io::desktop::windows_mutation::WindowsAlbumInspectionParentGrant::new(
                path.clone(), observed.identity.clone().ok_or_else(changed)?
            ).map_err(storage_error)?;
            let selection = RootSelection {
                token: self.next("recovery-parent"),
                display_name: path.to_string_lossy().into_owned(),
                capabilities: RootCapabilities { read: observed.capabilities.read == Some(true), list: observed.capabilities.list == Some(true) },
                platform: platform(),
            };
            let mut inner = self.lock()?;
            if !inner.recovery.records.iter().any(|record| record.token == token) { return Err(changed()); }
            inner.recovery.parents.retain(|root| root.record_token != token);
            inner.recovery.parents.push(ParentRoot { record_token: token.to_owned(), selection: selection.clone(), path, observed });
            Ok(selection)
        }
        pub fn select_recovery_path(&self, role: Role, path: PathBuf) -> Result<RootSelection> {
            validate_path(&path)?;
            let observed = inspect_root(&path)?;
            let selection = RootSelection {
                token: self.next("recovery-root"),
                display_name: path.to_string_lossy().into_owned(),
                capabilities: RootCapabilities {
                    read: observed.capabilities.read == Some(true),
                    list: observed.capabilities.list == Some(true),
                },
                platform: platform(),
            };
            let mut inner = self.lock()?;
            inner.recovery.roots.retain(|root| root.role != role);
            inner.recovery.roots.push(Root {
                role,
                selection: selection.clone(),
                path,
                observed,
            });
            Ok(selection)
        }
        #[cfg(test)]
        pub fn list_recovery(&self, offset: usize, limit: usize) -> Result<RecoveryPage> {
            self.list_recovery_with_control(offset, limit, None)
        }
        fn list_recovery_with_control(
            &self,
            offset: usize,
            limit: usize,
            control: Option<&InspectionControl>,
        ) -> Result<RecoveryPage> {
            checkpoint(control)?;
            if let Some(control) = control {
                control.stage("discovery").map_err(recorded_error)?;
            }
            if limit == 0 || limit > PAGE_LIMIT || offset > MAX_RECORDS {
                return Err(HostError::new("INVALID_PAGE", "עמוד יומנים אינו תקין."));
            }
            let directory = {
                let mut inner = self.lock()?;
                inner.recovery.records.clear();
                inner.recovery.roots.clear();
                inner.recovery.parents.clear();
                inner.journal_directory.clone().ok_or_else(|| {
                    HostError::new("JOURNAL_UNAVAILABLE", "תיקיית היומנים אינה זמינה.")
                })?
            };
            // Desktop pins the full root ancestry against replacement on Windows.
            // Raw read_dir only enumerates bounded names while those handles live;
            // every content read below is a bound, reparse-rejecting Desktop read.
            let mut storage = probe(control, || {
                Desktop::new(std::slice::from_ref(&directory)).map_err(storage_error)
            })?;
            let root_reference = probe(control, || {
                storage.reference(&directory).map_err(storage_error)
            })?;
            let root = probe(control, || {
                storage.stat(&root_reference).map_err(storage_error)
            })?;
            let mut names = Vec::new();
            let mut truncated = false;
            let mut entries = probe(control, || {
                std::fs::read_dir(&directory).map_err(storage_error)
            })?;
            for index in 0..=MAX_DIRECTORY_ENTRIES {
                let entry = probe(control, || {
                    entries.next().transpose().map_err(storage_error)
                })?;
                let Some(entry) = entry else {
                    break;
                };
                if index == MAX_DIRECTORY_ENTRIES {
                    truncated = true;
                    break;
                }
                if let Some(name) = entry
                    .file_name()
                    .to_str()
                    .filter(|name| journal_descriptor(name).is_some())
                {
                    names.push(name.to_owned());
                }
            }
            // A matching transfer journal is displayed with its directory journal.
            // An orphan remains visible instead of silently disappearing.
            let directory_names = names
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<_>>();
            names.retain(|name| {
                let (stem, family) = journal_descriptor(name).expect("filtered journal name");
                family != "orphan"
                    || (!directory_names.contains(&format!("{stem}.directories.jsonl"))
                        && !directory_names.contains(&format!("{stem}.album.jsonl")))
            });
            names.sort_by(|a, b| journal_order(b).cmp(&journal_order(a)));
            if names.len() > MAX_RECORDS {
                names.truncate(MAX_RECORDS);
                truncated = true;
            }
            if offset > names.len() {
                return Err(HostError::new(
                    "INVALID_PAGE",
                    "רשימת היומנים השתנתה. יש לרענן אותה.",
                ));
            }
            let total = names.len();
            let mut records = Vec::new();
            let mut retained = Vec::new();
            let mut budget = PAGE_BYTE_BUDGET;
            for name in names.into_iter().skip(offset).take(limit) {
                checkpoint(control)?;
                let token = self.next("recovery-record");
                let (stem, family) = journal_descriptor(&name).expect("filtered safe journal name");
                if name.ends_with(".album-move.jsonl") || family == "orphan" {
                    let regular = match observation(&mut storage, &directory.join(&name), control) {
                        Ok(observed) => observed.is_some_and(|entry| entry.kind == EntryKind::File),
                        Err(error) if error.code == "CANCELLED" => return Err(error),
                        Err(_) => false,
                    };
                    let detail = match family {
                        "album" => {
                            "נמצא יומן מיון אלבומים. בדיקה לאחר פתיחה מחדש עדיין אינה נתמכת לפעולה זו. יש לבדוק את תיקיות המקור והיעד לפני ביצוע נוסף; היומן נשמר."
                        }
                        "repair" => {
                            "נמצא יומן תיקון שמות ותגיות. בדיקה לאחר פתיחה מחדש עדיין אינה נתמכת לפעולה זו. יש לבדוק את הקבצים ולשמור את העותקים המקוריים והיומן לפני תיקון נוסף."
                        }
                        _ => {
                            "נמצא יומן העברות ללא יומן התיקיות התואם. לא ניתן לאמת את הפעולה. יש לשמור את היומן ולבדוק את תיקיות המקור והיעד לפני ביצוע נוסף."
                        }
                    };
                    records.push(RecoveryRecord {
                        token,
                        job_id: Some(stem.into()),
                        journal_name: name,
                        family,
                        source: None,
                        target: None,
                        original_parent: None,
                        state: if regular { "unsupported" } else { "invalid" },
                        detail: if regular {
                            detail.into()
                        } else {
                            "לא ניתן לאמת קובץ יומן רגיל במיקום זה. לא בוצעו שינויים.".into()
                        },
                        operation_count: 0,
                    });
                    continue;
                }
                let checked = (|| {
                    let observed = observation(&mut storage, &directory.join(&name), control)?
                        .ok_or_else(changed)?;
                    let (directories, directory_evidence) =
                        read_bound(&mut storage, &observed, &mut budget, control)?;
                    if family == "repair" {
                        let fallback =
                            InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
                        let descriptor =
                            inspect_recorded_repair_job(&directories, control.unwrap_or(&fallback))
                                .map_err(recorded_error)?;
                        if descriptor.job_id != stem {
                            return Err(changed());
                        }
                        let record = RecoveryRecord { token: token.clone(), job_id: Some(descriptor.job_id), journal_name: name.clone(), family,
                            source: Some(display(&descriptor.source)), target: None, original_parent: None, state: "inspection_needed",
                            detail: "נמצא יומן תיקון שמות ותגיות. יש לבחור מחדש את תיקיית המקור כדי לבדוק את הקבצים והעותקים המקוריים. היומן לבדו אינו מאמת את מצבם.".into(), operation_count: descriptor.operations.len() };
                        retained.push(RetainedRecord {
                            token: token.clone(),
                            directory: directory.clone(),
                            root: root.clone(),
                            journal: RetainedJournal::Repair {
                                journal: directory_evidence,
                            },
                        });
                        return Ok(record);
                    }
                    if family == "album" {
                        let fallback = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
                        let descriptor = inspect_recorded_album_file_job(&directories, control.unwrap_or(&fallback)).map_err(recorded_error)?;
                        if descriptor.job_id != stem { return Err(changed()); }
                        let transfer_path = directory.join(format!("{stem}.transfers.jsonl"));
                        let transfers = observation(&mut storage, &transfer_path, control)?
                            .map(|observed| read_bound(&mut storage, &observed, &mut budget, control))
                            .transpose()?;
                        let original_parent = descriptor.original_source.as_ref().and_then(|source| {
                            PathBuf::from(display(&source.reference)).parent().map(|path| path.to_string_lossy().into_owned())
                        });
                        let record = RecoveryRecord {
                            token: token.clone(), job_id: Some(descriptor.job_id.clone()), journal_name: name.clone(), family,
                            source: Some(display(&descriptor.source.reference)), target: Some(display(&descriptor.target.reference)),
                            original_parent, state: "inspection_needed",
                            detail: "נמצא יומן מיון אלבומים. יש לבחור מחדש את מיקום המקור והיעד כדי לבדוק את הקבצים. היומן לבדו אינו מאמת את מצבם.".into(),
                            operation_count: descriptor.file_count,
                        };
                        retained.push(RetainedRecord {
                            token: token.clone(), directory: directory.clone(), root: root.clone(),
                            journal: RetainedJournal::Album { manifest: directory_evidence, transfers: transfers.map(|(_, evidence)| evidence), transfer_path, descriptor },
                        });
                        return Ok(record);
                    }
                    let transfer_path = directory.join(format!("{stem}.transfers.jsonl"));
                    let transfer = observation(&mut storage, &transfer_path, control)?
                        .map(|observed| read_bound(&mut storage, &observed, &mut budget, control))
                        .transpose()?;
                    let summary = summarize_recorded_selected_job_with_control(
                        &directories,
                        transfer.as_ref().map(|(bytes, _)| bytes.as_slice()),
                        control,
                    )
                    .map_err(recorded_error)?;
                    if summary.job.job_id != stem {
                        return Err(changed());
                    }
                    let record = RecoveryRecord {
                        token: token.clone(),
                        job_id: Some(summary.job.job_id),
                        journal_name: name.clone(),
                        family,
                        source: Some(display(&summary.job.source)),
                        target: Some(display(&summary.job.target)),
                        original_parent: None,
                        state: if summary.recorded_complete {
                            "recorded_complete"
                        } else {
                            "inspection_needed"
                        },
                        detail: single_record_detail(summary.recorded_complete).into(),
                        operation_count: summary.job.operations.len(),
                    };
                    retained.push(RetainedRecord {
                        token: token.clone(),
                        directory: directory.clone(),
                        root: root.clone(),
                        journal: RetainedJournal::Single {
                            directories: directory_evidence,
                            transfers: transfer.map(|(_, evidence)| evidence),
                            transfer_path,
                        },
                    });
                    Ok(record)
                })();
                if let Err(error) = &checked {
                    if error.code == "CANCELLED" {
                        return Err(error.clone());
                    }
                }
                records.push(checked.unwrap_or_else(|error: HostError| RecoveryRecord {
                    token,
                    job_id: None,
                    journal_name: name,
                    family,
                    source: None,
                    target: None,
                    original_parent: None,
                    state: "invalid",
                    detail: format!("לא ניתן לאמת את היומן. {}", error.message),
                    operation_count: 0,
                }));
            }
            checkpoint(control)?;
            self.lock()?.recovery.records = retained;
            Ok(RecoveryPage {
                records,
                total,
                offset,
                truncated,
            })
        }
        #[cfg(test)]
        pub fn inspect_recovery(
            &self,
            token: &str,
            source_token: &str,
            target_token: &str,
        ) -> Result<RecoveryReport> {
            self.inspect_recovery_with_control(token, source_token, Some(target_token), None, 0)
        }
        fn inspect_recovery_with_control(
            &self,
            token: &str,
            source_token: &str,
            target_token: Option<&str>,
            control: Option<&InspectionControl>,
            cursor: usize,
        ) -> Result<RecoveryReport> {
            checkpoint(control)?;
            let (record, source, target) = {
                let inner = self.lock()?;
                let record = inner
                    .recovery
                    .records
                    .iter()
                    .find(|record| record.token == token)
                    .cloned()
                    .ok_or_else(changed)?;
                let find = |token: &str, role| {
                    inner
                        .recovery
                        .roots
                        .iter()
                        .find(|root| root.selection.token == token && root.role == role)
                        .cloned()
                        .ok_or_else(changed)
                };
                let repair = matches!(&record.journal, RetainedJournal::Repair { .. });
                let target = match (repair, target_token) {
                    (true, None) => None,
                    (false, Some(token)) => Some(find(token, Role::Target)?),
                    (true, Some(_)) => {
                        return Err(HostError::new(
                            "UNEXPECTED_RECOVERY_TARGET",
                            "בדיקת תיקון דורשת רק את תיקיית המקור.",
                        ));
                    }
                    (false, None) => {
                        return Err(HostError::new(
                            "MISSING_RECOVERY_TARGET",
                            "בדיקת מיון דורשת גם תיקיית יעד.",
                        ));
                    }
                };
                let source = if matches!(&record.journal, RetainedJournal::Album { .. }) {
                    match find(source_token, Role::Source) {
                        Ok(root) => SelectedSource::Present(root),
                        Err(_) => SelectedSource::Missing(inner.recovery.parents.iter()
                            .find(|parent| parent.record_token == record.token && parent.selection.token == source_token)
                            .cloned().ok_or_else(changed)?),
                    }
                } else {
                    SelectedSource::Present(find(source_token, Role::Source)?)
                };
                (record, source, target)
            };
            if matches!(&record.journal, RetainedJournal::Album { .. }) {
                return inspect_album_record(record, source, target.ok_or_else(changed)?, control, cursor);
            }
            let SelectedSource::Present(source) = source else { return Err(changed()); };
            if matches!(&record.journal, RetainedJournal::Repair { .. }) {
                return inspect_repair_record(record, source, control, cursor)
                    .map_err(repair_inspection_error);
            }
            let target = target.ok_or_else(changed)?;
            let RetainedJournal::Single {
                directories: directory_evidence,
                transfers: transfer_evidence,
                transfer_path,
            } = &record.journal
            else {
                return Err(changed());
            };
            let grant = |path, identity| {
                probe(control, || {
                    WindowsSelectedRootGrant::new(path, identity).map_err(storage_error)
                })
            };
            let roots = probe(control, || {
                WindowsMutationStorage::for_selected_roots(
                    grant(source.path, source.observed.identity.ok_or_else(changed)?)?,
                    grant(target.path, target.observed.identity.ok_or_else(changed)?)?,
                    grant(
                        record.directory.clone(),
                        record.root.identity.clone().ok_or_else(changed)?,
                    )?,
                )
                .map_err(storage_error)
            })?;
            let mut storage = probe(control, || {
                Desktop::new(std::slice::from_ref(&record.directory)).map_err(storage_error)
            })?;
            let mut budget = PAGE_BYTE_BUDGET;
            let directories =
                read_retained(&mut storage, directory_evidence, &mut budget, control)?;
            let transfers = match transfer_evidence {
                Some(evidence) => {
                    Some(read_retained(&mut storage, evidence, &mut budget, control)?)
                }
                None => {
                    if observation(&mut storage, transfer_path, control)?.is_some() {
                        return Err(changed());
                    }
                    None
                }
            };
            let summary = summarize_recorded_selected_job_with_control(
                &directories,
                transfers.as_deref(),
                control,
            )
            .map_err(recorded_error)?;
            let report = if let Some(control) = control {
                recover_selected_journals_controlled(&roots,&directories,transfers.as_deref(),control,cursor)
            } else { recover_selected_journals(&roots,&directories,transfers.as_deref()) }.map_err(|error| {
                if matches!(error, sorter_executor::ExecutionError::Cancelled | sorter_executor::ExecutionError::Unsupported) { recorded_error(error) }
                else { HostError::new("RECOVERY_CHECK_FAILED", format!("לא ניתן לאמת את הקבצים. יש לבחור את אותן תיקיות מקור ויעד שתועדו, ולוודא שהן זמינות. היומנים נשמרו. {error}")) }
            })?;
            let transfer_report = report.transfers.as_ref();
            let operations = summary
                .job
                .operations
                .iter()
                .map(|operation| {
                    let status = transfer_report
                        .and_then(|report| {
                            report
                                .operations
                                .iter()
                                .find(|item| item.operation == operation.id)
                        })
                        .map(|item| format!("{:?}", item.status))
                        .unwrap_or_else(|| "NeedsRecovery".into());
                    RecoveryOperation {
                        operation: operation.id,
                        source: display(&operation.source),
                        destination: format!(
                            "{}\\{}",
                            display(&operation.destination.root).trim_end_matches('\\'),
                            operation.destination.components.join("\\")
                        ),
                        status,
                        evidence: vec![],
                    }
                })
                .collect::<Vec<_>>();
            let consistent = !report.observation_limit_reached
                && report.directories.unverified.is_empty()
                && report.directories.complete
                && !report.directories.incomplete_tail
                && report.directories.pending.is_empty()
                && report.directories.rejected.is_empty()
                && transfer_report.is_some_and(|report| !report.append_blocked)
                && !operations.is_empty()
                && operations
                    .iter()
                    .all(|operation| operation.status == "Committed");
            checkpoint(control)?;
            Ok(RecoveryReport {
                job_id: summary.job.job_id,
                state: if consistent {
                    "consistent"
                } else {
                    "needs_attention"
                },
                observed_directories: report.directories.observed.len(),
                pending_directories: report.directories.pending.len(),
                rejected_directories: report.directories.rejected.len(),
                incomplete_tail: report.directories.incomplete_tail,
                unverified_directories: report.directories.unverified.len(),
                observation_limit_reached: report.observation_limit_reached,
                operations,
                evidence: vec![],
            })
        }
    }
    fn inspect_album_record(
        record: RetainedRecord,
        source: SelectedSource,
        target: Root,
        control: Option<&InspectionControl>,
        cursor: usize,
    ) -> Result<RecoveryReport> {
        project_album_inspection(read_album_inspection(record, source, target, control, cursor)?)
    }

    fn read_album_inspection(
        record: RetainedRecord,
        source: SelectedSource,
        target: Root,
        control: Option<&InspectionControl>,
        cursor: usize,
    ) -> Result<sorter_executor::windows_album::RecordedAlbumFileInspection> {
        use sorter_executor::windows_album::{
            AlbumFileInspectionGrants, AlbumFileInspectionJournal, AlbumFileInspectionSource,
            recover_recorded_album_file_job,
        };
        use sorter_io::desktop::windows_mutation::WindowsAlbumInspectionParentGrant;
        use sorter_io::mutation::MutationName;
        let fallback = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
        let control = control.unwrap_or(&fallback);
        let RetainedJournal::Album { manifest, transfers, transfer_path, descriptor } = &record.journal else {
            return Err(changed());
        };
        let mut storage = probe(Some(control), || Desktop::new(std::slice::from_ref(&record.directory)).map_err(storage_error))?;
        let reference = storage.reference(&record.directory).map_err(storage_error)?;
        let observed = probe(Some(control), || storage.stat(&reference).map_err(storage_error))?;
        if observed.identity != record.root.identity || observed.kind != EntryKind::Directory { return Err(changed()); }
        let mut budget = PAGE_BYTE_BUDGET;
        // Preserve discovery's exact metadata/digest evidence before native open.
        // The executor repeats the digest check through its own held read handles.
        read_retained(&mut storage, manifest, &mut budget, Some(control))?;
        if let Some(evidence) = transfers {
            read_retained(&mut storage, evidence, &mut budget, Some(control))?;
        } else if observation(&mut storage, transfer_path, Some(control))?.is_some() {
            return Err(changed());
        }
        drop(storage);
        let journal = |evidence: &JournalEvidence| -> Result<AlbumFileInspectionJournal> {
            Ok(AlbumFileInspectionJournal {
                name: MutationName::Windows(evidence.observed.name.encode_utf16().collect()),
                identity: evidence.observed.identity.clone().ok_or_else(changed)?,
                length: evidence.observed.size.ok_or_else(changed)?,
                sha256: evidence.sha256.clone(),
            })
        };
        let mut journals = vec![journal(manifest)?];
        if let Some(evidence) = transfers { journals.push(journal(evidence)?); }
        let grant = |root: Root| WindowsSelectedRootGrant::new(root.path, root.observed.identity.ok_or_else(changed)?).map_err(storage_error);
        let source = match source {
            SelectedSource::Present(root) => AlbumFileInspectionSource::Present(grant(root)?),
            SelectedSource::Missing(parent) => AlbumFileInspectionSource::Missing(
                WindowsAlbumInspectionParentGrant::new(parent.path, parent.observed.identity.ok_or_else(changed)?).map_err(storage_error)?
            ),
        };
        let grants = AlbumFileInspectionGrants {
            source, target: grant(target)?,
            journal: WindowsSelectedRootGrant::new(record.directory, record.root.identity.ok_or_else(changed)?).map_err(storage_error)?,
        };
        recover_recorded_album_file_job(descriptor, grants, journals, control, cursor).map_err(recorded_error)
    }

    fn project_album_inspection(
        inspected: sorter_executor::windows_album::RecordedAlbumFileInspection,
    ) -> Result<RecoveryReport> {
        use sorter_executor::recovery::{CopyMetadataRecoveryStatus, RecoveryStatus};
        use sorter_executor::windows_album::{AlbumDirectoryEvidenceStatus, AlbumInspectionSourceState};
        use std::collections::HashSet;
        let recorded = inspected.recorded;
        let report = inspected.recovery;
        if report.whole.is_some() || recorded.files.len() != recorded.file_count
            || inspected.files.len() != recorded.files.len()
        { return Err(changed()); }
        let mut operations = Vec::with_capacity(inspected.files.len());
        for (index, item) in inspected.files.into_iter().enumerate() {
            if item.file != recorded.files[index] || item.file.physical_operation != index
                || item.recovery.as_ref().is_some_and(|entry| entry.operation != index)
            { return Err(changed()); }
            let status = item.recovery.as_ref().map(|entry| {
                if matches!(entry.copy_metadata, CopyMetadataRecoveryStatus::Verified | CopyMetadataRecoveryStatus::NotRequested) {
                    format!("{:?}", entry.status)
                } else { "NeedsRecovery".into() }
            }).unwrap_or_else(|| "NeedsRecovery".into());
            let evidence = item.recovery.as_ref().filter(|entry| entry.copy_metadata != CopyMetadataRecoveryStatus::NotRequested)
                .map(|entry| RecoveryEvidence {
                    role: "file_metadata".into(), path: display(&item.file.destination),
                    status: match entry.copy_metadata {
                        CopyMetadataRecoveryStatus::Verified => "Verified",
                        CopyMetadataRecoveryStatus::Mismatch => "Changed",
                        _ => "Unverified",
                    }.into(),
                }).into_iter().collect();
            operations.push(RecoveryOperation { operation: index, source: display(&item.file.source),
                destination: display(&item.file.destination), status, evidence });
        }
        let removed = report.removed_directory_references.iter().map(|(reference, identity)| (reference, identity)).collect::<HashSet<_>>();
        if removed.len() != report.removed_directory_references.len() { return Err(changed()); }
        let root_removed = removed.contains(&(&recorded.source.reference, recorded.source.identity.as_ref().ok_or_else(changed)?));
        let mut evidence = vec![match inspected.source_state {
            AlbumInspectionSourceState::Present { observed } => RecoveryEvidence {
                role: "source_root".into(), path: display(&observed.reference),
                status: if root_removed { "Changed" } else { "Verified" }.into(),
            },
            AlbumInspectionSourceState::Absent { recorded: original } => RecoveryEvidence {
                role: "source_root".into(), path: display(&original.reference),
                status: if root_removed { "AbsentExpected" } else { "Missing" }.into(),
            },
        }];
        evidence.push(RecoveryEvidence { role: "target_root".into(), path: display(&recorded.target.reference), status: "Verified".into() });
        let mut observed_directories = 0;
        let mut rejected_directories = 0;
        let mut unverified_directories = 0;
        let mut observed_removed = HashSet::new();
        for directory in &report.directories {
            let was_removed = removed.contains(&(&directory.reference, &directory.expected_identity));
            let status = match directory.status {
                AlbumDirectoryEvidenceStatus::Absent if was_removed => {
                    observed_removed.insert((&directory.reference, &directory.expected_identity));
                    observed_directories += 1; "AbsentExpected"
                }
                AlbumDirectoryEvidenceStatus::SameIdentity if !was_removed
                    && directory.observed_identity.as_ref() == Some(&directory.expected_identity) => {
                    observed_directories += 1; "Verified"
                }
                AlbumDirectoryEvidenceStatus::Unverified => { unverified_directories += 1; "Unverified" }
                AlbumDirectoryEvidenceStatus::Absent => { rejected_directories += 1; "Missing" }
                _ => { rejected_directories += 1; "Changed" }
            };
            evidence.push(RecoveryEvidence { role: if was_removed { "removed_directory" } else { "directory" }.into(), path: display(&directory.reference), status: status.into() });
        }
        for (reference, identity) in &report.removed_directory_references {
            if !observed_removed.contains(&(reference, identity)) {
                evidence.push(RecoveryEvidence { role: "removed_directory".into(), path: display(reference), status: "Unverified".into() });
            }
        }
        let completed = report.completed_albums.iter().copied().collect::<HashSet<_>>();
        let expected = recorded.album_operations.iter().copied().collect::<HashSet<_>>();
        let transfer_complete = report.transfers.as_ref().is_some_and(|transfers| {
            !transfers.append_blocked && transfers.operations.len() == recorded.files.len()
                && transfers.operations.iter().all(|entry| entry.status == RecoveryStatus::Committed
                    && matches!(entry.copy_metadata, CopyMetadataRecoveryStatus::Verified | CopyMetadataRecoveryStatus::NotRequested))
        });
        let terminal_complete = report.manifest_finished && report.recorded_status.as_deref() == Some("Complete")
            && !report.incomplete_tail && transfer_complete && completed == expected
            && completed.len() == report.completed_albums.len();
        evidence.push(RecoveryEvidence { role: "journal_completion".into(), path: format!("{}.album.jsonl", recorded.job_id), status: if terminal_complete { "Verified" } else { "Unverified" }.into() });
        let pending = report.pending_directories.iter().chain(&report.pending_cleanup).chain(&report.pending_metadata).collect::<HashSet<_>>();
        for reference in &pending {
            evidence.push(RecoveryEvidence { role: "pending_directory".into(), path: display(reference), status: "Unverified".into() });
        }
        for (reference, status) in &report.directory_metadata_status {
            evidence.push(RecoveryEvidence { role: "directory_metadata".into(), path: display(reference), status: match status.as_str() { "verified" => "Verified", "changed" | "mismatch" => "Changed", _ => "Unverified" }.into() });
        }
        let consistent = terminal_complete && pending.is_empty() && !operations.is_empty()
            && report.directory_metadata_status.len() == report.directory_metadata.len()
            && observed_removed.len() == removed.len()
            && evidence.iter().all(|entry| matches!(entry.status.as_str(), "Verified" | "AbsentExpected"))
            && operations.iter().all(|entry| entry.status == "Committed");
        Ok(RecoveryReport {
            job_id: recorded.job_id, state: if consistent { "consistent" } else { "needs_attention" },
            observed_directories, pending_directories: pending.len(), rejected_directories,
            incomplete_tail: report.incomplete_tail, unverified_directories, observation_limit_reached: false,
            operations, evidence,
        })
    }

    fn repair_inspection_error(error: HostError) -> HostError {
        if matches!(
            error.code.as_str(),
            "CANCELLED" | "RECOVERY_LIMIT" | "JOURNAL_LIMIT" | "JOURNAL_CHANGED"
        ) {
            return error;
        }
        HostError::new(
            "RECOVERY_CHECK_FAILED",
            "לא ניתן לאמת את התיקון. הבדיקה לא שינתה קבצים. יש לשמור את היומן ולבדוק את תיקיית המקור שתועדה ואת העותקים המקוריים.",
        )
    }
    fn inspect_repair_record(
        record: RetainedRecord,
        source: Root,
        control: Option<&InspectionControl>,
        cursor: usize,
    ) -> Result<RecoveryReport> {
        let fallback = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
        let controlled = control.unwrap_or(&fallback);
        checkpoint(Some(controlled))?;
        let grant = |path, identity| {
            probe(Some(controlled), || {
                WindowsSelectedRootGrant::new(path, identity).map_err(storage_error)
            })
        };
        let roots = probe(Some(controlled), || {
            WindowsMutationStorage::for_selected_repair_roots(
                grant(source.path, source.observed.identity.ok_or_else(changed)?)?,
                grant(
                    record.directory.clone(),
                    record.root.identity.clone().ok_or_else(changed)?,
                )?,
            )
            .map_err(storage_error)
        })?;
        let RetainedJournal::Repair { journal } = &record.journal else {
            return Err(changed());
        };
        let mut storage = probe(Some(controlled), || {
            Desktop::new(std::slice::from_ref(&record.directory)).map_err(storage_error)
        })?;
        let mut budget = PAGE_BYTE_BUDGET;
        let bytes = read_retained(&mut storage, journal, &mut budget, Some(controlled))?;
        let descriptor = inspect_recorded_repair_job(&bytes, controlled).map_err(recorded_error)?;
        let report = recover_repair_journal_controlled(
            &roots,
            &bytes,
            &AtomicBool::new(false),
            controlled,
            cursor,
        )
        .map_err(recorded_error)?;
        let mut operations = Vec::new();
        for item in &report.operations {
            checkpoint(Some(controlled))?;
            let original = descriptor
                .operations
                .get(item.operation)
                .ok_or_else(changed)?;
            let verified = item
                .recorded
                .as_ref()
                .is_some_and(|recorded| recorded.status == RepairStatus::Complete)
                && !item.evidence.is_empty()
                && item
                    .evidence
                    .iter()
                    .all(|evidence| matches!(evidence.status, RepairEvidenceStatus::Verified));
            let original_path = display(&original.source);
            let intended = original_path.rfind(['\\', '/']).map_or_else(
                || original.intended_name.clone(),
                |index| format!("{}{}", &original_path[..=index], original.intended_name),
            );
            let destination = item
                .evidence
                .iter()
                .find(|evidence| evidence.role == "current")
                .map(|evidence| display(&evidence.reference))
                .unwrap_or(intended);
            let evidence = item
                .evidence
                .iter()
                .map(|evidence| RecoveryEvidence {
                    role: evidence.role.clone(),
                    path: display(&evidence.reference),
                    status: format!("{:?}", evidence.status),
                })
                .collect();
            operations.push(RecoveryOperation {
                operation: item.operation,
                source: original_path,
                destination,
                status: if verified {
                    "Committed"
                } else {
                    "NeedsRecovery"
                }
                .into(),
                evidence,
            });
        }
        let consistent = report.recorded_finished
            && !report.incomplete_tail
            && !operations.is_empty()
            && operations
                .iter()
                .all(|operation| operation.status == "Committed");
        checkpoint(Some(controlled))?;
        Ok(RecoveryReport {
            job_id: report.job_id,
            state: if consistent {
                "consistent"
            } else {
                "needs_attention"
            },
            observed_directories: 0,
            pending_directories: 0,
            rejected_directories: 0,
            incomplete_tail: report.incomplete_tail,
            unverified_directories: 0,
            observation_limit_reached: false,
            operations,
            evidence: vec![],
        })
    }
    #[cfg(test)]
    include!("recovery_album_tests.rs");

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn inspection_needed_descriptor_does_not_claim_missing_terminal_records() {
            // Copied-Move descriptors deliberately keep this flag false even
            // for a valid final journal; it expresses inspection policy only.
            let detail = single_record_detail(false);
            assert!(detail.contains("לבחור מחדש"));
            assert!(detail.contains("היומן לבדו אינו מאמת"));
            assert!(!detail.contains("לא נמצא סיום"));
            assert!(!detail.contains("לא הושלם"));
            assert!(single_record_detail(true).contains("דורש בדיקה"));
        }
        #[test]
        fn journal_names_are_only_single_generated_components() {
            assert_eq!(
                journal_descriptor("sorter-1-2-3.directories.jsonl"),
                Some(("sorter-1-2-3", "single"))
            );
            assert_eq!(
                journal_descriptor("album-1-2-3.album.jsonl"),
                Some(("album-1-2-3", "album"))
            );
            assert_eq!(
                journal_descriptor("album-1-2-3.album-move.jsonl"),
                Some(("album-1-2-3", "album"))
            );
            assert_eq!(
                journal_descriptor("album-1-2-3.transfers.jsonl"),
                Some(("album-1-2-3", "orphan"))
            );
            assert_eq!(
                journal_descriptor("repair-1-2-3.repair.jsonl"),
                Some(("repair-1-2-3", "repair"))
            );
            for name in [
                "../sorter-1.directories.jsonl",
                "sorter-x\\y.directories.jsonl",
                "other.directories.jsonl",
                "sorter-1:stream.directories.jsonl",
            ] {
                assert!(journal_descriptor(name).is_none());
            }
        }
        #[test]
        fn history_order_uses_generated_time_before_process_identifier() {
            assert!(
                journal_order("sorter-1-200-0.directories.jsonl")
                    > journal_order("sorter-9999-100-0.directories.jsonl")
            );
        }
        #[test]
        fn bounded_reads_reject_oversize_and_changed_journal_evidence() {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("sorter-1.directories.jsonl");
            std::fs::write(&path, b"first").unwrap();
            let mut storage = Desktop::new(&[directory.path().into()]).unwrap();
            let observed = observation(&mut storage, &path, None).unwrap().unwrap();
            let mut budget = PAGE_BYTE_BUDGET;
            let (_, evidence) = read_bound(&mut storage, &observed, &mut budget, None).unwrap();
            std::fs::write(&path, b"other").unwrap();
            assert!(read_retained(&mut storage, &evidence, &mut budget, None).is_err());
            let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
            file.set_len(MAX_RESTART_JOURNAL_BYTES as u64 + 1).unwrap();
            drop(file);
            let observed = observation(&mut storage, &path, None).unwrap().unwrap();
            assert_eq!(
                read_bound(&mut storage, &observed, &mut budget, None)
                    .err()
                    .unwrap()
                    .code,
                "JOURNAL_LIMIT"
            );
        }
        #[test]
        fn fresh_host_discovers_and_reconciles_completed_job_without_writes() {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("source");
            let target = directory.path().join("target");
            let journals = directory.path().join("journals");
            for path in [&source, &target, &journals] {
                std::fs::create_dir(path).unwrap();
            }
            let bytes = std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../tests/fixtures/media/tagged.mp3"),
            )
            .unwrap();
            let song = source.join("אברהם פריד - בדיקת התאוששות.mp3");
            std::fs::write(&song, &bytes).unwrap();
            let host = Host::default();
            host.set_journal_directory(journals.clone()).unwrap();
            let s = host.select_path(Role::Source, source.clone()).unwrap();
            let t = host.select_path(Role::Target, target.clone()).unwrap();
            let settings = JobSettings {
                copy_mode: true,
                main_folder_only: false,
                singles_folder: false,
                exist_only: false,
                abc_sort: false,
                duet_mode: false,
            };
            // The real command always includes profile bytes, even when empty.
            let id = host
                .preview_with_personal(
                    PreviewRequest {
                        mode: JobMode::Sort,
                        source_token: s.token,
                        target_token: Some(t.token),
                        settings,
                        ai_profile: RequestedAi::Disabled,
                    },
                    Some(vec![]),
                )
                .unwrap()
                .job_id;
            let snapshot = host.snapshot(&id).unwrap();
            assert!(
                snapshot.apply.as_ref().unwrap().supported,
                "{:?}",
                snapshot.apply
            );
            host.start_apply(
                &id,
                snapshot.plan_id.as_ref().unwrap(),
                vec![],
                host.claim().unwrap(),
            )
            .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                let result = host.execution_snapshot(&id).unwrap();
                if !result.is_running() {
                    assert_eq!(result.status, "complete", "{:?}", result);
                    break;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            while host.shared.busy.load(Ordering::Acquire) {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            drop(host); // No retained plan/scope/tokens survive the owning process.
            let before = std::fs::read_dir(&journals)
                .unwrap()
                .map(|entry| {
                    let path = entry.unwrap().path();
                    let bytes = std::fs::read(&path).unwrap();
                    (path, bytes)
                })
                .collect::<Vec<_>>();
            let restarted = Host::default();
            restarted.set_journal_directory(journals).unwrap();
            let page = restarted.list_recovery(0, 10).unwrap();
            assert_eq!(page.records.len(), 1);
            assert_eq!(page.records[0].state, "recorded_complete");
            assert!(
                restarted
                    .inspect_recovery(&page.records[0].token, "unknown", "unknown")
                    .is_err()
            );
            let s = restarted
                .select_recovery_path(Role::Source, source)
                .unwrap();
            let t = restarted
                .select_recovery_path(Role::Target, target.clone())
                .unwrap();
            let report = restarted
                .inspect_recovery(&page.records[0].token, &s.token, &t.token)
                .unwrap();
            assert_eq!(report.state, "consistent");
            assert_eq!(report.operations.len(), 1);
            assert_eq!(std::fs::read(&song).unwrap(), bytes);
            std::fs::write(
                target.join("אברהם פריד").join(song.file_name().unwrap()),
                b"external change",
            )
            .unwrap();
            let changed = restarted
                .inspect_recovery(&page.records[0].token, &s.token, &t.token)
                .unwrap();
            assert_eq!(changed.state, "needs_attention");
            for (path, bytes) in before {
                assert_eq!(std::fs::read(path).unwrap(), bytes);
            }
        }
        pub(super) fn wait_for_recovery(host: &Host, id: &str) -> RecoverySnapshot {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                let snapshot = host.recovery_snapshot(id).unwrap();
                if !matches!(snapshot.status, "running" | "cancelling") {
                    return snapshot;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        pub(super) fn owned_tree_state(
            root: &Path,
        ) -> Vec<(PathBuf, EntryObservation, u32, u64, u64, Option<String>)> {
            use std::os::windows::fs::MetadataExt;
            let mut reads = Desktop::new(&[root.to_path_buf()]).unwrap();
            let mut pending = vec![root.to_path_buf()];
            let mut values = Vec::new();
            while let Some(path) = pending.pop() {
                let metadata = std::fs::symlink_metadata(&path).unwrap();
                assert!(!metadata.file_type().is_symlink());
                let reference = reads.reference(&path).unwrap();
                let observed = reads.stat(&reference).unwrap();
                let content = metadata.is_file().then(|| {
                    let mut file = std::fs::File::open(&path).unwrap();
                    let mut hash = Sha256::new();
                    let mut chunk = [0u8; 64 * 1024];
                    loop {
                        let count = file.read(&mut chunk).unwrap();
                        if count == 0 { break; }
                        hash.update(&chunk[..count]);
                    }
                    format!("{:x}", hash.finalize())
                });
                if metadata.is_dir() {
                    pending.extend(
                        std::fs::read_dir(&path)
                            .unwrap()
                            .map(|entry| entry.unwrap().path()),
                    );
                }
                values.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    observed,
                    metadata.file_attributes(),
                    metadata.creation_time(),
                    metadata.last_write_time(),
                    content,
                ));
            }
            values.sort_by(|left, right| left.0.cmp(&right.0));
            values
        }
        #[test]
        fn actual_content_chunk_close_cancels_and_fresh_host_reopens_read_only() {
            let fixture = tempfile::tempdir().unwrap();
            let source = fixture.path().join("source");
            let target = fixture.path().join("target");
            let journals = fixture.path().join("journals");
            for path in [&source, &target, &journals] {
                std::fs::create_dir(path).unwrap();
            }
            let bytes = std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../tests/fixtures/media/tagged.mp3"),
            )
            .unwrap();
            std::fs::write(source.join("אברהם פריד - בדיקת עצירה.mp3"), &bytes).unwrap();
            let host = Host::default();
            host.set_journal_directory(journals.clone()).unwrap();
            let selected_source = host.select_path(Role::Source, source.clone()).unwrap();
            let selected_target = host.select_path(Role::Target, target.clone()).unwrap();
            let settings = JobSettings {
                copy_mode: true,
                main_folder_only: false,
                singles_folder: false,
                exist_only: false,
                abc_sort: false,
                duet_mode: false,
            };
            // The real command always includes profile bytes, even when empty.
            let id = host
                .preview_with_personal(
                    PreviewRequest {
                        mode: JobMode::Sort,
                        source_token: selected_source.token,
                        target_token: Some(selected_target.token),
                        settings,
                        ai_profile: RequestedAi::Disabled,
                    },
                    Some(vec![]),
                )
                .unwrap()
                .job_id;
            let preview = host.snapshot(&id).unwrap();
            host.start_apply(
                &id,
                preview.plan_id.as_ref().unwrap(),
                vec![],
                host.claim().unwrap(),
            )
            .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                let execution = host.execution_snapshot(&id).unwrap();
                if !execution.is_running() && !host.shared.busy.load(Ordering::Acquire) {
                    assert_eq!(execution.status, "complete", "{execution:?}");
                    break;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            drop(host);
            let before = owned_tree_state(fixture.path());
            let reopened = Host::default();
            reopened.set_journal_directory(journals.clone()).unwrap();
            let listed = reopened
                .start_recovery_list(0, 10, reopened.claim().unwrap())
                .unwrap();
            let page = wait_for_recovery(&reopened, &listed.run_id).page.unwrap();
            assert_eq!(page.records.len(), 1);
            let token = page.records[0].token.clone();
            let source_token = reopened
                .select_recovery_path(Role::Source, source.clone())
                .unwrap()
                .token;
            let target_token = reopened
                .select_recovery_path(Role::Target, target.clone())
                .unwrap()
                .token;
            let actual_chunk = Arc::new(AtomicBool::new(false));
            let saw_chunk = actual_chunk.clone();
            let identity = Some((
                token.clone(),
                source_token.clone(),
                Some(target_token.clone()),
            ));
            let run = reopened
                .start_recovery_run(
                    "inspect",
                    identity,
                    None,
                    reopened.claim().unwrap(),
                    move |cancel, host, id, cursor| {
                        let close_host = host.clone();
                        let progress_host = host.clone();
                        let progress_id = id.to_owned();
                        let media_start = AtomicU64::new(u64::MAX);
                        let control = InspectionControl::with_progress_interval(
                            cancel.cancelled.clone(),
                            std::time::Duration::ZERO,
                            move |progress| {
                                if progress.stage == "files" {
                                    let _ = media_start.compare_exchange(
                                        u64::MAX,
                                        progress.bytes_read,
                                        Ordering::AcqRel,
                                        Ordering::Acquire,
                                    );
                                    if progress.bytes_read > media_start.load(Ordering::Acquire) {
                                        saw_chunk.store(true, Ordering::Release);
                                        close_host.cancel_active();
                                    }
                                }
                                progress_host
                                    .update_recovery_progress(&progress_id, progress.into());
                            },
                        );
                        let result = host
                            .inspect_recovery_with_control(
                                &token,
                                &source_token,
                                Some(&target_token),
                                Some(&control),
                                cursor,
                            )
                            .map(RecoveryOutput::Report);
                        host.update_recovery_progress(
                            id,
                            control.snapshot().map_err(recorded_error)?.into(),
                        );
                        result
                    },
                )
                .unwrap();
            let stopped = wait_for_recovery(&reopened, &run.run_id);
            assert_eq!(stopped.status, "cancelled");
            assert!(actual_chunk.load(Ordering::Acquire));
            assert_eq!(stopped.progress.completed_operations, 0);
            assert!(stopped.report.is_none());
            assert_eq!(owned_tree_state(fixture.path()), before);
            drop(reopened);
            // Process ownership and all retained tokens are gone; fresh choices
            // must read every file again before a complete result is possible.
            let fresh = Host::default();
            fresh.set_journal_directory(journals).unwrap();
            let listed = fresh
                .start_recovery_list(0, 10, fresh.claim().unwrap())
                .unwrap();
            let page = wait_for_recovery(&fresh, &listed.run_id).page.unwrap();
            let source = fresh.select_recovery_path(Role::Source, source).unwrap();
            let target = fresh.select_recovery_path(Role::Target, target).unwrap();
            let run = fresh
                .start_recovery_inspection(
                    page.records[0].token.clone(),
                    source.token,
                    Some(target.token),
                    None,
                    fresh.claim().unwrap(),
                )
                .unwrap();
            let checked = wait_for_recovery(&fresh, &run.run_id);
            assert_eq!(checked.status, "completed");
            assert_eq!(checked.report.unwrap().state, "consistent");
            assert_eq!(checked.progress.completed_operations, 1);
            assert_eq!(owned_tree_state(fixture.path()), before);
        }
        #[test]
        fn failed_fix_inspection_does_not_claim_missing_or_changed_backups_survived() {
            for detail in ["Stale", "Io", "Scope", "CorruptJournal"] {
                let error = repair_inspection_error(HostError::new("STORAGE_ERROR", detail));
                assert_eq!(error.code, "RECOVERY_CHECK_FAILED");
                assert!(error.message.contains("לא שינתה קבצים"));
                assert!(error.message.contains("לבדוק"));
                assert!(!error.message.contains("נשמרו"));
                assert!(!error.message.contains(detail));
            }
        }
        #[test]
        fn fresh_fix_restart_requires_source_only_and_verifies_retained_backups() {
            let fixture = tempfile::tempdir().unwrap();
            let source = fixture.path().join("source");
            let wrong = fixture.path().join("wrong-source");
            let journals = fixture.path().join("journals");
            for path in [&source, &wrong, &journals] {
                std::fs::create_dir(path).unwrap();
            }
            let fixture_bytes = std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../tests/fixtures/media/tagged.mp3"),
            )
            .unwrap();
            let tag_length = fixture_bytes[6..10]
                .iter()
                .fold(0usize, |length, byte| (length << 7) | usize::from(*byte));
            let mut body = Vec::new();
            for field in [b"TPE1", b"TPE2", b"TIT2", b"TALB", b"TCON"] {
                body.extend(field);
                body.extend([0, 0, 0, 6, 0, 0, 0, 0xe0, 0xe1, 0xf8, 0xe4, 0xed]);
            }
            body.extend([0u8; 256]);
            let length = body.len();
            let mut bytes = b"ID3\x04\0\0".to_vec();
            bytes.extend([
                ((length >> 21) & 127) as u8,
                ((length >> 14) & 127) as u8,
                ((length >> 7) & 127) as u8,
                (length & 127) as u8,
            ]);
            bytes.extend(body);
            bytes.extend(&fixture_bytes[10 + tag_length..]);
            std::fs::write(source.join("song__demo.mp3"), &bytes).unwrap();
            let host = Host::default();
            host.set_journal_directory(journals.clone()).unwrap();
            let selected = host.select_path(Role::Source, source.clone()).unwrap();
            let settings = JobSettings {
                copy_mode: false,
                main_folder_only: true,
                singles_folder: false,
                exist_only: false,
                abc_sort: false,
                duet_mode: false,
            };
            // The real command always includes profile bytes, even when empty.
            let id = host
                .preview_with_personal(
                    PreviewRequest {
                        mode: JobMode::FixNames,
                        source_token: selected.token,
                        target_token: None,
                        settings,
                        ai_profile: RequestedAi::Disabled,
                    },
                    Some(vec![]),
                )
                .unwrap()
                .job_id;
            let preview = host.snapshot(&id).unwrap();
            assert!(preview.apply.as_ref().unwrap().supported);
            host.start_apply(
                &id,
                preview.plan_id.as_ref().unwrap(),
                vec![],
                host.claim().unwrap(),
            )
            .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                let execution = host.execution_snapshot(&id).unwrap();
                if !execution.is_running() && !host.shared.busy.load(Ordering::Acquire) {
                    assert_eq!(execution.status, "complete", "{execution:?}");
                    break;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            drop(host);
            let before = owned_tree_state(fixture.path());
            let fresh = Host::default();
            fresh.set_journal_directory(journals).unwrap();
            let listed = fresh
                .start_recovery_list(0, 10, fresh.claim().unwrap())
                .unwrap();
            let page = wait_for_recovery(&fresh, &listed.run_id).page.unwrap();
            assert_eq!(page.records.len(), 1);
            let record = &page.records[0];
            assert_eq!(
                (record.family, record.state),
                ("repair", "inspection_needed")
            );
            assert!(record.target.is_none());
            let wrong = fresh.select_recovery_path(Role::Source, wrong).unwrap();
            let run = fresh
                .start_recovery_inspection(
                    record.token.clone(),
                    wrong.token,
                    None,
                    None,
                    fresh.claim().unwrap(),
                )
                .unwrap();
            assert_eq!(wait_for_recovery(&fresh, &run.run_id).status, "failed");
            let selected = fresh
                .select_recovery_path(Role::Source, source.clone())
                .unwrap();
            let run = fresh
                .start_recovery_inspection(
                    record.token.clone(),
                    selected.token.clone(),
                    Some("unexpected-target".into()),
                    None,
                    fresh.claim().unwrap(),
                )
                .unwrap();
            assert_eq!(
                wait_for_recovery(&fresh, &run.run_id).error.unwrap().code,
                "UNEXPECTED_RECOVERY_TARGET"
            );
            let run = fresh
                .start_recovery_inspection(
                    record.token.clone(),
                    selected.token.clone(),
                    None,
                    None,
                    fresh.claim().unwrap(),
                )
                .unwrap();
            let report = wait_for_recovery(&fresh, &run.run_id).report.unwrap();
            assert_eq!(report.state, "consistent");
            assert!(
                report.operations[0]
                    .evidence
                    .iter()
                    .any(|evidence| evidence.role == "backup" && evidence.status == "Verified")
            );
            assert_eq!(owned_tree_state(fixture.path()), before);
            let backup = std::fs::read_dir(&source)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| path.extension().is_some_and(|ext| ext == "backup"))
                .unwrap();
            std::fs::write(backup, b"externally changed original backup").unwrap();
            let changed = owned_tree_state(fixture.path());
            let run = fresh
                .start_recovery_inspection(
                    record.token.clone(),
                    selected.token,
                    None,
                    None,
                    fresh.claim().unwrap(),
                )
                .unwrap();
            let report = wait_for_recovery(&fresh, &run.run_id).report.unwrap();
            assert_eq!(report.state, "needs_attention");
            assert!(
                report.operations[0]
                    .evidence
                    .iter()
                    .any(|evidence| evidence.role == "backup" && evidence.status == "Changed")
            );
            assert_eq!(owned_tree_state(fixture.path()), changed);
        }
        #[test]
        fn unsupported_and_orphan_journals_stay_visible_without_recovery_authority() {
            let directory = tempfile::tempdir().unwrap();
            for name in [
                "album-1-20-1.album.jsonl",
                "album-1-21-1.album-move.jsonl",
                "album-1-22-1.transfers.jsonl",
                "repair-1-30-1.repair.jsonl",
                "sorter-1-40-1.transfers.jsonl",
            ] {
                std::fs::write(directory.path().join(name), b"untrusted journal bytes").unwrap();
            }
            let host = Host::default();
            host.set_journal_directory(directory.path().into()).unwrap();
            let page = host.list_recovery(0, 10).unwrap();
            assert_eq!(page.records.len(), 5);
            assert!(page.records.iter().all(|record| record.state
                == if record.family == "orphan" || record.journal_name.ends_with(".album-move.jsonl") {
                    "unsupported"
                } else {
                    "invalid"
                }));
            assert!(host.lock().unwrap().recovery.records.is_empty());
            for record in page.records {
                assert!(
                    host.inspect_recovery(&record.token, "source", "target")
                        .is_err()
                );
                assert_eq!(
                    std::fs::read(directory.path().join(record.journal_name)).unwrap(),
                    b"untrusted journal bytes"
                );
            }
        }
        #[test]
        fn parent_picker_requires_a_retained_schema_two_album_record() {
            let directory = tempfile::tempdir().unwrap();
            let host = Host::default();
            assert!(host.select_recovery_parent_path("unretained", directory.path().into()).is_err());
            assert!(host.lock().unwrap().recovery.parents.is_empty());
            assert!(std::fs::read_dir(directory.path()).unwrap().next().is_none());
        }
        #[test]
        fn recovery_root_tokens_never_replace_preview_selection() {
            let directory = tempfile::tempdir().unwrap();
            let host = Host::default();
            let preview = host
                .select_path(Role::Source, directory.path().into())
                .unwrap();
            let recovered = host
                .select_recovery_path(Role::Source, directory.path().into())
                .unwrap();
            assert_ne!(preview.token, recovered.token);
            let inner = host.lock().unwrap();
            assert_eq!(inner.roots[0].selection.token, preview.token);
            assert_eq!(inner.recovery.roots[0].selection.token, recovered.token);
        }
    }
}
