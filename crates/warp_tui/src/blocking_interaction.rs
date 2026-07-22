//! Centralized state bookkeeping for which entity (if any) is currently blocking the input
//! for a given session.

use warpui_core::{Entity, EntityId, ModelContext};

/// Emitted whenever the active blocker changes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TuiBlockingInteractionEvent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TuiBlockerAlreadyActive {
    pub(crate) blocker: EntityId,
}

/// The sole interaction currently blocking normal session input.
///
/// Concrete views retain ownership of rendering, placement, focus, and
/// lifecycle behavior. This model stores only their opaque identity.
pub(crate) struct TuiBlockingInteractionModel {
    blocker: Option<EntityId>,
}

impl TuiBlockingInteractionModel {
    pub(crate) fn new(_: &mut ModelContext<Self>) -> Self {
        Self { blocker: None }
    }

    pub(crate) fn blocker(&self) -> Option<EntityId> {
        self.blocker
    }

    pub(crate) fn is_active(&self) -> bool {
        self.blocker.is_some()
    }

    /// Activates `blocker` when no other interaction owns the state.
    pub(crate) fn activate(
        &mut self,
        blocker: EntityId,
        ctx: &mut ModelContext<Self>,
    ) -> Result<(), TuiBlockerAlreadyActive> {
        match self.blocker {
            Some(active) if active == blocker => return Ok(()),
            Some(active) => return Err(TuiBlockerAlreadyActive { blocker: active }),
            None => {}
        }
        self.blocker = Some(blocker);
        ctx.emit(TuiBlockingInteractionEvent);
        ctx.notify();
        Ok(())
    }

    /// Clears `blocker` only when it still owns the centralized state. Returns
    /// whether the blocker was cleared. A stale teardown from an older
    /// interaction cannot clear a newer blocker.
    pub(crate) fn deactivate(&mut self, blocker: EntityId, ctx: &mut ModelContext<Self>) -> bool {
        if self.blocker != Some(blocker) {
            return false;
        }

        self.blocker = None;
        ctx.emit(TuiBlockingInteractionEvent);
        ctx.notify();
        true
    }
}

impl Entity for TuiBlockingInteractionModel {
    type Event = TuiBlockingInteractionEvent;
}

#[cfg(test)]
#[path = "blocking_interaction_tests.rs"]
mod tests;
