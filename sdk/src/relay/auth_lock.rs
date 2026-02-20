use tokio::sync::{Semaphore, SemaphorePermit};

const MAX_READS: u32 = u32::MAX >> 3;

#[derive(Debug)]
pub(super) struct AuthLock {
    semaphore: Semaphore,
}

impl AuthLock {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            semaphore: Semaphore::new(MAX_READS as usize),
        }
    }

    #[cfg(test)]
    fn new_with_permits(permits: u32) -> Self {
        Self {
            semaphore: Semaphore::new(permits as usize),
        }
    }

    /// Acquire one permit for regular message sending.
    #[inline]
    pub(super) async fn acquire_message_permit(&self) -> SemaphorePermit<'_> {
        tracing::debug!("acquiring message permit");
        self.semaphore
            .acquire()
            .await
            .expect("semaphore should not be closed")
    }

    /// Acquire all permits for authentication, blocking message sending.
    #[inline]
    pub(super) async fn acquire_auth_guard(&self) -> SemaphorePermit<'_> {
        tracing::debug!("acquiring auth guard");
        self.semaphore
            .acquire_many(MAX_READS)
            .await
            .expect("semaphore should not be closed")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_utility::time;

    use super::*;

    #[tokio::test]
    async fn message_permit_blocks_when_all_message_permits_are_taken() {
        let lock = AuthLock::new_with_permits(2);

        let first = lock.acquire_message_permit().await;
        let _second = lock.acquire_message_permit().await;

        let third_attempt = time::timeout(
            Some(Duration::from_millis(50)),
            lock.acquire_message_permit(),
        )
        .await;
        assert!(third_attempt.is_none());

        drop(first);

        let third = time::timeout(Some(Duration::from_secs(1)), lock.acquire_message_permit())
            .await
            .expect("single permit should become available after releasing one");
        drop(third);
    }

    #[tokio::test]
    async fn auth_guard_waits_for_messages_and_then_blocks_new_messages() {
        let lock = AuthLock::new();

        // Simulate a message send in progress.
        let in_flight_send = lock.acquire_message_permit().await;

        // Authentication must wait until all in-flight sends finish.
        let auth_while_send_in_flight =
            time::timeout(Some(Duration::from_millis(50)), lock.acquire_auth_guard()).await;
        assert!(auth_while_send_in_flight.is_none());

        // Once send finishes, authentication can acquire the full lock.
        drop(in_flight_send);
        let auth_guard = time::timeout(Some(Duration::from_secs(1)), lock.acquire_auth_guard())
            .await
            .expect("auth lock should eventually be acquired");

        // While auth is in progress, new sends must be blocked.
        let send_during_auth = time::timeout(
            Some(Duration::from_millis(50)),
            lock.acquire_message_permit(),
        )
        .await;
        assert!(send_during_auth.is_none());

        // When auth finishes, sends can proceed again.
        drop(auth_guard);
        let send_after_auth =
            time::timeout(Some(Duration::from_secs(1)), lock.acquire_message_permit())
                .await
                .expect("send should continue once auth releases the lock");
        drop(send_after_auth);
    }
}
