use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub device_info: Option<String>,
    pub user_agent: Option<String>,
}

impl Session {
    pub fn new(device_info: Option<String>, user_agent: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            last_activity: now,
            device_info,
            user_agent,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    pub fn is_expired(&self, timeout_seconds: i64) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_activity);
        elapsed.num_seconds() > timeout_seconds
    }
}

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    timeout_seconds: i64,
}

impl SessionManager {
    pub fn new(timeout_seconds: i64) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            timeout_seconds,
        }
    }

    pub async fn create_session(
        &self,
        device_info: Option<String>,
        user_agent: Option<String>,
    ) -> Session {
        let session = Session::new(device_info, user_agent);
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session.id.clone(), session.clone());
        session
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).cloned()
    }

    pub async fn update_activity(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.update_activity();
            true
        } else {
            false
        }
    }

    pub async fn remove_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id).is_some()
    }

    pub async fn list_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.lock().await;
        sessions.values().cloned().collect()
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.lock().await;
        let initial_count = sessions.len();
        sessions.retain(|_, session| !session.is_expired(self.timeout_seconds));
        initial_count - sessions.len()
    }

    pub async fn active_count(&self) -> usize {
        let sessions = self.sessions.lock().await;
        sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new(1800);
        let session = manager
            .create_session(Some("test-device".to_string()), None)
            .await;
        assert!(!session.id.is_empty());
        assert_eq!(manager.active_count().await, 1);
    }

    #[tokio::test]
    async fn test_session_activity_update() {
        let manager = SessionManager::new(1800);
        let session = manager.create_session(None, None).await;
        let session_id = session.id.clone();

        sleep(Duration::from_millis(100)).await;
        assert!(manager.update_activity(&session_id).await);

        let updated = manager.get_session(&session_id).await.unwrap();
        assert!(updated.last_activity > session.created_at);
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let manager = SessionManager::new(1); // 1 second timeout
        let session = manager.create_session(None, None).await;

        assert!(!session.is_expired(1));
        sleep(Duration::from_secs(2)).await;
        assert!(session.is_expired(1));
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let manager = SessionManager::new(1);
        manager.create_session(None, None).await;
        manager.create_session(None, None).await;

        assert_eq!(manager.active_count().await, 2);
        sleep(Duration::from_secs(2)).await;

        let removed = manager.cleanup_expired().await;
        assert_eq!(removed, 2);
        assert_eq!(manager.active_count().await, 0);
    }
}
