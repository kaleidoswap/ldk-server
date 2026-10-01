use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Mutex;

/// Auto-payment is opt-in. Unknown or replayed invoice events remain unpaid.
pub(crate) struct AutoPayGate<I> {
	ids: Mutex<HashSet<I>>,
}

impl<I: Eq + Hash + Copy> AutoPayGate<I> {
	pub(crate) fn new() -> Self {
		Self { ids: Mutex::new(HashSet::new()) }
	}

	/// Serialize event classification with the synchronous request initiation.
	pub(crate) fn authorize<E>(&self, start: impl FnOnce() -> Result<I, E>) -> Result<I, E> {
		let mut ids = self.ids.lock().unwrap();
		let id = start()?;
		ids.insert(id);
		Ok(id)
	}

	pub(crate) fn take(&self, id: &I) -> bool {
		self.ids.lock().unwrap().remove(id)
	}
}

#[cfg(test)]
mod tests {
	use super::AutoPayGate;
	use std::sync::{mpsc, Arc};
	use std::thread;

	#[test]
	fn fetch_unknown_restart_and_replayed_events_never_auto_pay() {
		let gate = AutoPayGate::<u64>::new();
		assert!(!gate.take(&7));
		gate.authorize(|| Ok::<_, ()>(7)).unwrap();
		assert!(gate.take(&7));
		assert!(!gate.take(&7));
		assert!(!AutoPayGate::<u64>::new().take(&7));
	}

	#[test]
	fn failed_request_does_not_authorize() {
		let gate = AutoPayGate::<u64>::new();
		assert_eq!(gate.authorize(|| Err::<u64, _>("failed")), Err("failed"));
		assert!(!gate.take(&7));
	}

	#[test]
	fn invoice_arriving_during_start_waits_for_explicit_authorization() {
		let gate = Arc::new(AutoPayGate::<u64>::new());
		let (started_tx, started_rx) = mpsc::channel();
		let (event_tx, event_rx) = mpsc::channel();
		let event_gate = Arc::clone(&gate);
		let event = thread::spawn(move || {
			started_rx.recv().unwrap();
			event_tx.send(()).unwrap();
			event_gate.take(&42)
		});
		gate.authorize(|| {
			assert!(gate.ids.try_lock().is_err());
			started_tx.send(()).unwrap();
			event_rx.recv().unwrap();
			Ok::<_, ()>(42)
		})
		.unwrap();
		assert!(event.join().unwrap());
		assert!(!gate.take(&42));
	}
}
