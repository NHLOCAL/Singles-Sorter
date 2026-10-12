//! Ordered, exact direct-child renames inside a held album. The caller supplies
//! the frozen planner order, including unchanged leaves; native code preserves
//! that order and never infers policy from filesystem enumeration order.
use super::*;
use std::os::windows::io::IntoRawHandle;
use windows::Win32::Globalization::CompareStringOrdinal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsAlbumChildRename {
    pub source: MutationName,
    pub destination: MutationName,
    pub identity: ObjectIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsAlbumChildRenameObservation {
    /// Index in the original complete planner order, including unchanged leaves.
    pub planner_index: usize,
    pub rename: WindowsAlbumChildRename,
    pub observation: MutationObservation,
    pub entries: Vec<WindowsAlbumTreeObservation>,
}

/// A separate quiescent-album scope. It never exposes a mutable parent grant.
/// Even after an uncertain child rename, the original root remains held here.
pub struct WindowsAlbumChildRenameScope {
    source: WindowsAlbumDirectoryLease,
    ordered: Vec<WindowsAlbumChildRename>,
    cursor: usize,
    stopped: bool,
}
impl WindowsAlbumChildRenameScope {
    pub fn observation(&self) -> &MutationObservation {
        self.source.observation()
    }
    pub fn ordered_children(&self) -> &[WindowsAlbumChildRename] {
        &self.ordered
    }
    pub fn current_tree(&self) -> &[WindowsAlbumTreeEntry] {
        &self.source.tree.entries
    }
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }
    pub fn is_complete(&self) -> bool {
        !self.stopped && self.cursor == self.ordered.len()
    }
    pub fn next_child_rename(&self) -> Option<(usize, &WindowsAlbumChildRename)> {
        if self.stopped {
            None
        } else {
            self.ordered
                .get(self.cursor)
                .map(|item| (self.cursor, item))
        }
    }
    /// An incomplete/stopped conversion returns the whole scope, retaining its
    /// root pin for partial-outcome inspection. No fallible handle transition.
    pub fn into_album_directory_lease(self) -> Result<WindowsAlbumDirectoryLease, Box<Self>> {
        if self.is_complete() {
            Ok(self.source)
        } else {
            Err(Box::new(self))
        }
    }
    fn advance_unchanged(&mut self) {
        while self
            .ordered
            .get(self.cursor)
            .is_some_and(|item| item.source == item.destination)
        {
            self.cursor += 1;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChildRenameCheckpoint {
    BeforeFinalVerification,
    BeforeRename,
    AfterRename,
    AfterClose,
    AfterVerification,
}

impl WindowsMutationStorage {
    pub fn acquire_album_directory_for_child_renames(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        expected: &ObjectIdentity,
        entries: Vec<WindowsAlbumTreeEntry>,
        ordered: Vec<WindowsAlbumChildRename>,
        cancel: &AtomicBool,
    ) -> Result<WindowsAlbumChildRenameScope, MutationError> {
        let source = self.acquire_album_directory(parent, name, expected, entries, cancel)?;
        bind_child_renames(source, ordered, cancel)
    }

    /// The caller must durably journal the exact next item before calling this.
    /// There is no caller-supplied leaf or index which can bypass the cursor.
    pub fn rename_next_album_child(
        &self,
        scope: &mut WindowsAlbumChildRenameScope,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumChildRenameObservation> {
        self.rename_next_album_child_with(scope, cancel, (false, |_| Ok(()), close_child_once))
    }

    /// Forward a complete cursor to the existing whole-container operation,
    /// retaining the scope's original-location ancestry for reconciliation.
    pub fn rename_completed_album_children(
        &self,
        scope: &mut WindowsAlbumChildRenameScope,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumPublication> {
        self.rename_completed_album_children_with(scope, parent, name, cancel, (false, |_| Ok(())))
    }
    fn rename_completed_album_children_with(
        &self,
        scope: &mut WindowsAlbumChildRenameScope,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
        execution: (
            bool,
            impl FnMut(AlbumRenameCheckpoint) -> Result<(), MutationError>,
        ),
    ) -> MutationOutcome<WindowsAlbumPublication> {
        if !scope.is_complete() {
            return MutationOutcome::NoEffect(err(
                MutationErrorKind::StalePlan,
                "album child cursor is incomplete or stopped",
            ));
        }
        let outcome =
            self.rename_album_directory_with(&mut scope.source, parent, name, cancel, execution);
        if matches!(&outcome, MutationOutcome::PendingUnknown(_)) {
            scope.stopped = true;
        }
        outcome
    }

    pub fn inspect_original_album_child_rename_location(
        &self,
        scope: &WindowsAlbumChildRenameScope,
    ) -> Result<Option<MutationObservation>, MutationError> {
        self.inspect_original_album_location(&scope.source)
    }

    /// Reverify the last observed manifest through the retained root, including
    /// for a stopped scope. This does not accept an uncertain rename as success.
    pub fn verify_album_child_rename_tree(
        &self,
        scope: &WindowsAlbumChildRenameScope,
        cancel: &AtomicBool,
    ) -> Result<Vec<WindowsAlbumTreeObservation>, MutationError> {
        verify_child_scope_tree(self, scope, false, cancel)
    }

    /// Probe only one original binding's source or destination leaf. A foreign
    /// identity is evidence, not authority to change it. None means NotFound.
    pub fn inspect_album_child_rename_location(
        &self,
        scope: &WindowsAlbumChildRenameScope,
        planner_index: usize,
        at_destination: bool,
    ) -> Result<Option<MutationObservation>, MutationError> {
        inspect_child_scope_location(self, scope, false, planner_index, at_destination)
    }

    fn rename_next_album_child_with(
        &self,
        scope: &mut WindowsAlbumChildRenameScope,
        cancel: &AtomicBool,
        execution: (
            bool,
            impl FnMut(ChildRenameCheckpoint) -> Result<(), MutationError>,
            impl FnOnce(File) -> Result<(), MutationError>,
        ),
    ) -> MutationOutcome<WindowsAlbumChildRenameObservation> {
        let (root_session, mut checkpoint, close) = execution;
        let preflight = (|| {
            cancelled(cancel)?;
            if self.domain == WindowsMutationDomain::SelectedRepair {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "repair cannot rename album children",
                ));
            }
            scope.source.scope.check(self, root_session)?;
            let (index, rename) = scope.next_child_rename().ok_or_else(|| {
                err(
                    MutationErrorKind::StalePlan,
                    "child rename scope stopped or complete",
                )
            })?;
            let rename = rename.clone();
            let root = scope
                .source
                .file
                .as_ref()
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "album root consumed"))?;
            checkpoint(ChildRenameCheckpoint::BeforeFinalVerification)?;
            let observed = album_observe(root, scope.source.scope.parent_identity())?;
            if observed.identity != scope.source.original.identity
                || observed.actual_name != scope.source.original.actual_name
            {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "album root changed before child rename",
                )
                .with_observation(observed));
            }
            verify_album_tree(root, &scope.source.tree, cancel)?;
            require_absent_child(root, &rename.destination)?;
            let candidate = renamed_tree(&scope.source, &rename)?;
            let entry = scope
                .source
                .tree
                .entries
                .iter()
                .find(|entry| entry.relative.len() == 1 && entry.relative[0] == rename.source)
                .ok_or_else(|| {
                    err(
                        MutationErrorKind::StalePlan,
                        "next child is missing from current tree",
                    )
                })?;
            if entry.identity != rename.identity {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "next child identity differs from bound order",
                ));
            }
            let child = open_album_relative(
                root,
                units(&rename.source)?,
                DELETE | FILE_GENERIC_READ,
                FILE_WRITE_THROUGH
                    | if entry.kind == EntryKind::Directory {
                        FILE_DIRECTORY_FILE
                    } else {
                        FILE_NON_DIRECTORY_FILE
                    },
                FILE_SHARE_MODE(0),
            )?;
            let observation = album_observe(&child, &scope.source.original.identity)?;
            if observation.identity != rename.identity
                || observation.actual_name != rename.source
                || kind(&child.metadata().map_err(io_error)?) != entry.kind
            {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "opened child differs from bound identity/name/kind",
                )
                .with_observation(observation));
            }
            if let Some(digest) = &entry.digest {
                ensure_regular(&child)?;
                if album_digest(&child, digest.length, cancel)? != *digest {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "held child content differs before rename",
                    ));
                }
            } else {
                ensure_directory(&child)?;
            }
            scope.source.scope.check(self, root_session)?;
            checkpoint(ChildRenameCheckpoint::BeforeRename)?;
            cancelled(cancel)?;
            Ok((index, rename, candidate, child))
        })();
        let (index, rename, candidate, child) = match preflight {
            Ok(value) => value,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        // Every dispatched attempt terminates the scope until all postconditions
        // pass. Native alias/collision failures cannot be blindly retried either.
        scope.stopped = true;
        // A simple leaf with a NULL RootDirectory is documented to rename only
        // within this exact child's current parent. The child denies DELETE
        // sharing; the held root cannot be moved. No full-path target is used.
        let dispatch = rename_album_handle_relative(&child, None, &rename.destination);
        if let Err(error) = dispatch {
            if error.kind != MutationErrorKind::UnknownOutcome {
                return MutationOutcome::NoEffect(error);
            }
            let _ = close(child);
            return child_pending(error, scope.observation());
        }
        let after_rename = checkpoint(ChildRenameCheckpoint::AfterRename);
        // Close the same child once even if the response/checkpoint was lost.
        // Verification must reopen descendants, so retaining this share0 child
        // would prevent it. Never reconstruct or retry this numerical handle.
        let closed = close(child);
        let after = (|| {
            after_rename?;
            closed?;
            checkpoint(ChildRenameCheckpoint::AfterClose)?;
            cancelled(cancel)?;
            scope.source.scope.check(self, root_session)?;
            let root = scope.source.file.as_ref().expect("retained album root");
            let entries = verify_album_tree(root, &candidate, cancel)?;
            let observation = entries
                .iter()
                .find(|entry| entry.relative.len() == 1 && entry.relative[0] == rename.destination)
                .ok_or_else(|| {
                    err(
                        MutationErrorKind::VerificationFailed,
                        "renamed child observation missing",
                    )
                })?
                .observation
                .clone();
            if observation.identity != rename.identity
                || observation.actual_name != rename.destination
            {
                return Err(err(
                    MutationErrorKind::VerificationFailed,
                    "renamed child observation differs",
                ));
            }
            checkpoint(ChildRenameCheckpoint::AfterVerification)?;
            cancelled(cancel)?;
            scope.source.scope.check(self, root_session)?;
            Ok((observation, entries))
        })();
        match after {
            Ok((observation, entries)) => {
                scope.source.tree = candidate;
                scope.cursor += 1;
                scope.advance_unchanged();
                scope.stopped = false;
                MutationOutcome::Applied(WindowsAlbumChildRenameObservation {
                    planner_index: index,
                    rename,
                    observation,
                    entries,
                })
            }
            Err(error) => child_pending(error, scope.observation()),
        }
    }
}

fn check_child_scope_root(
    storage: &WindowsMutationStorage,
    scope: &WindowsAlbumChildRenameScope,
    root_session: bool,
) -> Result<(), MutationError> {
    scope.source.scope.check(storage, root_session)?;
    let root = scope
        .source
        .file
        .as_ref()
        .ok_or_else(|| err(MutationErrorKind::StalePlan, "album root consumed"))?;
    ensure_directory(root)?;
    let observed = album_observe(root, scope.source.scope.parent_identity())?;
    if observed.identity != scope.source.original.identity
        || observed.actual_name != scope.source.original.actual_name
    {
        return Err(
            err(MutationErrorKind::StalePlan, "retained album root changed")
                .with_observation(observed),
        );
    }
    Ok(())
}
fn verify_child_scope_tree(
    storage: &WindowsMutationStorage,
    scope: &WindowsAlbumChildRenameScope,
    root_session: bool,
    cancel: &AtomicBool,
) -> Result<Vec<WindowsAlbumTreeObservation>, MutationError> {
    cancelled(cancel)?;
    check_child_scope_root(storage, scope, root_session)?;
    let entries = verify_album_tree(
        scope.source.file.as_ref().expect("checked root"),
        &scope.source.tree,
        cancel,
    )?;
    check_child_scope_root(storage, scope, root_session)?;
    cancelled(cancel)?;
    Ok(entries)
}
fn inspect_child_scope_location(
    storage: &WindowsMutationStorage,
    scope: &WindowsAlbumChildRenameScope,
    root_session: bool,
    planner_index: usize,
    at_destination: bool,
) -> Result<Option<MutationObservation>, MutationError> {
    check_child_scope_root(storage, scope, root_session)?;
    let binding = scope.ordered.get(planner_index).ok_or_else(|| {
        err(
            MutationErrorKind::InvalidName,
            "child observation index is outside the bound planner order",
        )
    })?;
    let leaf = if at_destination {
        &binding.destination
    } else {
        &binding.source
    };
    let root = scope.source.file.as_ref().expect("checked root");
    let result = match super::super::super::open_child(root, units(leaf)?, false) {
        Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => None,
        Err(error) => return Err(read_error(error)),
        Ok(file) => {
            if kind(&file.metadata().map_err(io_error)?) == EntryKind::LinkOrReparse {
                return Err(err(
                    MutationErrorKind::UnsafeReparse,
                    "bound child location is now reparse",
                ));
            }
            Some(album_observe(&file, &scope.source.original.identity)?)
        }
    };
    check_child_scope_root(storage, scope, root_session)?;
    Ok(result)
}

fn child_pending(
    error: MutationError,
    root: &MutationObservation,
) -> MutationOutcome<WindowsAlbumChildRenameObservation> {
    MutationOutcome::PendingUnknown(err(MutationErrorKind::UnknownOutcome,
        &format!("album child rename dispatched; original container retained, reconcile without replay: {error:?}"))
        .with_observation(root.clone()))
}

fn close_child_once(file: File) -> Result<(), MutationError> {
    let handle = HANDLE(file.into_raw_handle());
    unsafe { windows::Win32::Foundation::CloseHandle(handle) }.map_err(win_error)
}

fn require_absent_child(root: &File, name: &MutationName) -> Result<(), MutationError> {
    match super::super::super::open_child(root, units(name)?, false) {
        Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(()),
        Err(error) => Err(read_error(error)),
        Ok(_) => Err(err(
            MutationErrorKind::StalePlan,
            "child rename target is occupied or aliases an existing leaf",
        )),
    }
}

fn renamed_tree(
    source: &WindowsAlbumDirectoryLease,
    rename: &WindowsAlbumChildRename,
) -> Result<AlbumTree, MutationError> {
    let mut entries = source.tree.entries.clone();
    for entry in &mut entries {
        if entry.relative[0] == rename.source {
            entry.relative[0] = rename.destination.clone();
        }
    }
    validate_album_tree(&source.original.identity, entries)
}

fn bind_child_renames(
    source: WindowsAlbumDirectoryLease,
    ordered: Vec<WindowsAlbumChildRename>,
    cancel: &AtomicBool,
) -> Result<WindowsAlbumChildRenameScope, MutationError> {
    let expected = source
        .tree
        .children
        .get(&Vec::new())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if ordered.len() != expected.len() {
        return Err(err(
            MutationErrorKind::StalePlan,
            "child order must cover every direct child, including unchanged leaves",
        ));
    }
    let mut remaining = BTreeMap::new();
    for index in expected {
        let entry = &source.tree.entries[*index];
        remaining.insert(units(&entry.relative[0])?.to_vec(), &entry.identity);
    }
    for rename in &ordered {
        cancelled(cancel)?;
        units(&rename.destination)?;
        if remaining.remove(units(&rename.source)?) != Some(&rename.identity) {
            return Err(err(
                MutationErrorKind::StalePlan,
                "child order repeats, omits or changes an exact source identity",
            ));
        }
    }
    reject_ordinal_collisions(&ordered, cancel)?;
    let root = source.file.as_ref().expect("new album lease");
    for rename in ordered
        .iter()
        .filter(|item| item.source != item.destination)
    {
        cancelled(cancel)?;
        require_absent_child(root, &rename.destination)?;
    }
    verify_album_tree(root, &source.tree, cancel)?;
    let mut scope = WindowsAlbumChildRenameScope {
        source,
        ordered,
        cursor: 0,
        stopped: false,
    };
    scope.advance_unchanged();
    Ok(scope)
}

// Fallible merge sort of indices keeps the complete policy order untouched and
// bounds admission to O(n log n), rather than pairwise comparisons of big albums.
fn reject_ordinal_collisions(
    ordered: &[WindowsAlbumChildRename],
    cancel: &AtomicBool,
) -> Result<(), MutationError> {
    let compare = |a: usize, b: usize| -> Result<i32, MutationError> {
        cancelled(cancel)?;
        let result = unsafe {
            CompareStringOrdinal(
                units(&ordered[a].destination)?,
                units(&ordered[b].destination)?,
                true,
            )
        }
        .0;
        match result {
            1..=3 => Ok(result),
            _ => Err(io_error(io::Error::last_os_error())),
        }
    };
    let mut indices: Vec<_> = (0..ordered.len()).collect();
    let mut buffer = indices.clone();
    let mut width = 1usize;
    while width < indices.len() {
        for start in (0..indices.len()).step_by(width * 2) {
            cancelled(cancel)?;
            let middle = (start + width).min(indices.len());
            let end = (middle + width).min(indices.len());
            let (mut left, mut right) = (start, middle);
            for slot in &mut buffer[start..end] {
                if left < middle && (right == end || compare(indices[left], indices[right])? <= 2) {
                    *slot = indices[left];
                    left += 1;
                } else {
                    *slot = indices[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(&mut indices, &mut buffer);
        width *= 2;
    }
    for pair in indices.windows(2) {
        cancelled(cancel)?;
        if compare(pair[0], pair[1])? == 2 {
            return Err(err(
                MutationErrorKind::StalePlan,
                "planned child destinations are ordinal case-insensitive aliases",
            ));
        }
    }
    Ok(())
}

pub struct WindowsAlbumRootChildRenameSession {
    target_and_journal: WindowsMutationStorage,
    source_reference: StorageRef,
    scope: WindowsAlbumChildRenameScope,
}
impl WindowsAlbumRootChildRenameSession {
    pub fn selected_source_reference(&self) -> &StorageRef {
        &self.source_reference
    }
    pub fn selected_source_identity(&self) -> &ObjectIdentity {
        &self.scope.observation().identity
    }
    pub fn source_observation(&self) -> &MutationObservation {
        self.scope.observation()
    }
    pub fn target_and_journal(&self) -> &WindowsMutationStorage {
        &self.target_and_journal
    }
    pub fn ordered_children(&self) -> &[WindowsAlbumChildRename] {
        self.scope.ordered_children()
    }
    pub fn current_tree(&self) -> &[WindowsAlbumTreeEntry] {
        self.scope.current_tree()
    }
    pub fn is_stopped(&self) -> bool {
        self.scope.is_stopped()
    }
    pub fn is_complete(&self) -> bool {
        self.scope.is_complete()
    }
    pub fn next_child_rename(&self) -> Option<(usize, &WindowsAlbumChildRename)> {
        self.scope.next_child_rename()
    }
    pub fn rename_next_album_child(
        &mut self,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumChildRenameObservation> {
        self.target_and_journal.rename_next_album_child_with(
            &mut self.scope,
            cancel,
            (true, |_| Ok(()), close_child_once),
        )
    }
    pub fn rename_completed_album_children(
        &mut self,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumPublication> {
        self.target_and_journal
            .rename_completed_album_children_with(
                &mut self.scope,
                parent,
                name,
                cancel,
                (true, |_| Ok(())),
            )
    }
    pub fn inspect_original_location(&self) -> Result<Option<MutationObservation>, MutationError> {
        self.scope
            .source
            .scope
            .check(&self.target_and_journal, true)?;
        inspect_album_original_location(&self.scope.source)
    }
    pub fn verify_current_tree(
        &self,
        cancel: &AtomicBool,
    ) -> Result<Vec<WindowsAlbumTreeObservation>, MutationError> {
        verify_child_scope_tree(&self.target_and_journal, &self.scope, true, cancel)
    }
    pub fn inspect_child_location(
        &self,
        planner_index: usize,
        at_destination: bool,
    ) -> Result<Option<MutationObservation>, MutationError> {
        inspect_child_scope_location(
            &self.target_and_journal,
            &self.scope,
            true,
            planner_index,
            at_destination,
        )
    }
    pub fn into_root_rename_session(self) -> Result<WindowsAlbumRootRenameSession, Box<Self>> {
        if self.scope.is_complete() {
            Ok(WindowsAlbumRootRenameSession {
                target_and_journal: self.target_and_journal,
                source_reference: self.source_reference,
                source: self.scope.source,
            })
        } else {
            Err(Box::new(self))
        }
    }
}
impl WindowsMutationStorage {
    pub fn for_selected_album_root_child_renames(
        source: WindowsSelectedRootGrant,
        target: WindowsSelectedRootGrant,
        journal: WindowsSelectedRootGrant,
        entries: Vec<WindowsAlbumTreeEntry>,
        ordered: Vec<WindowsAlbumChildRename>,
        cancel: &AtomicBool,
    ) -> Result<WindowsAlbumRootChildRenameSession, MutationError> {
        let session =
            Self::for_selected_album_root_rename(source, target, journal, entries, cancel)?;
        let scope = bind_child_renames(session.source, ordered, cancel)?;
        Ok(WindowsAlbumRootChildRenameSession {
            target_and_journal: session.target_and_journal,
            source_reference: session.source_reference,
            scope,
        })
    }
}

#[cfg(test)]
#[path = "windows_album_child_rename_tests.rs"]
mod tests;
