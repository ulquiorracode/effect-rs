//! Integration bridge between `stitch-core` (SMA U-cycle) and `machine-core` (FSM / Effects).

use crate::{Machine, Step};

/// Adapter wrapping any `machine_core::Machine` as a `stitch_core::Terminal` execution handler.
///
/// This bridges the Sewing Machine Architecture's "Point of Puncture" with stateful FSM transitions:
/// - Each pipeline U-cycle descent reaches this terminal with an intent `TIntent` which acts as `Machine::Resume`.
/// - The terminal steps the state machine:
///   - If `Step::Yielded(y)`: maps to `Ok(TOutcome)` or returns intermediate yield.
///   - If `Step::Done(r)`: returns the terminal outcome.
pub struct MachineTerminalAdapter<M, F> {
    machine: M,
    outcome_mapper: F,
}

impl<M, F> MachineTerminalAdapter<M, F> {
    /// Creates a new `MachineTerminalAdapter`.
    #[inline(always)]
    pub const fn new(machine: M, outcome_mapper: F) -> Self {
        Self {
            machine,
            outcome_mapper,
        }
    }

    /// Access the underlying machine.
    #[inline(always)]
    pub fn machine(&self) -> &M {
        &self.machine
    }

    /// Mutable access to the underlying machine.
    #[inline(always)]
    pub fn machine_mut(&mut self) -> &mut M {
        &mut self.machine
    }
}

/// Transition outcome emitted by a stepped machine terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineTransition<Y, R> {
    /// Machine yielded an intermediate state / effect.
    Yielded(Y),
    /// Machine reached its terminal state.
    Done(R),
}

impl<TCtx, M, F, TOutcome, TErr> stitch_core::Terminal<TCtx, M::Resume, TOutcome, TErr>
    for MachineTerminalAdapter<M, F>
where
    M: Machine,
    F: FnMut(Step<M::Yield, M::Return>, &mut TCtx) -> Result<TOutcome, TErr>,
{
    #[inline(always)]
    fn execute(&mut self, ctx: &mut TCtx, intent: M::Resume) -> Result<TOutcome, TErr> {
        let step = self.machine.step(intent);
        (self.outcome_mapper)(step, ctx)
    }
}
