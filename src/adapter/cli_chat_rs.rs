//! cli-chat-rs MessagingAdapter blueprint for julesctl.
//! See: https://github.com/rvoidex7/cli-chat-rs/blob/main/ADAPTER_GUIDE.md
//!
//! Uncomment and adjust method signatures to match your cli-chat-rs version.

use crate::api::JulesClient;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

struct Inner {
    client: JulesClient,
    session_id: Option<String>,              // Optional for global mode
    project_sessions: Vec<(String, String)>, // (Session ID, Label/Title)
    _last_activity_name: Option<String>,
}

pub struct JulesAdapter {
    inner: Arc<Mutex<Inner>>,
}

impl JulesAdapter {
    pub fn new(api_key: &str, session_id: &str, title: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                client: JulesClient::new(api_key),
                session_id: Some(session_id.to_string()),
                project_sessions: vec![(session_id.to_string(), title.to_string())],
                _last_activity_name: None,
            })),
        }
    }
}

use async_trait::async_trait;
use cli_chat_rs::{
    AdapterResult, Chat, ChatId, ConnectionStatus, Contact, ContactId, Message, MessageContent,
    MessageId, MessageStatus, MessagingAdapter,
};

#[async_trait]
impl MessagingAdapter for JulesAdapter {
    fn name(&self) -> &str {
        "Jules AI"
    }

    async fn connect(&mut self) -> AdapterResult<()> {
        Ok(())
    }

    async fn disconnect(&mut self) -> AdapterResult<()> {
        Ok(())
    }

    fn connection_status(&self) -> ConnectionStatus {
        ConnectionStatus::Connected
    }

    async fn get_chats(&self) -> AdapterResult<Vec<Chat>> {
        let inner = self.inner.lock().await;

        // If project_sessions are defined (i.e. we are in a project context with specific sessions)
        if !inner.project_sessions.is_empty() {
            let mut chats = Vec::new();
            for (id, label) in &inner.project_sessions {
                chats.push(Chat {
                    id: id.clone(),
                    name: label.clone(),
                    is_group: false,
                    participants: vec!["jules".to_string()],
                    last_message: None,
                    unread_count: 0,
                });
            }
            return Ok(chats);
        }

        // Otherwise (global mode), list all sessions from Jules API
        let sessions = inner
            .client
            .list_sessions()
            .await
            .map_err(|e| e.to_string())?;
        Ok(sessions
            .into_iter()
            .map(|s| Chat {
                id: s.id().to_string(),
                name: s.title.clone(),
                is_group: false,
                participants: vec!["jules".to_string()],
                last_message: None, // Could be fetched but let's keep it simple for now
                unread_count: 0,
            })
            .collect())
    }

    async fn get_messages(&self, chat_id: &ChatId, limit: usize) -> AdapterResult<Vec<Message>> {
        let inner = self.inner.lock().await;
        let activities = inner
            .client
            .get_activities(chat_id, limit as u32)
            .await
            .map_err(|e| e.to_string())?;

        Ok(activities
            .into_iter()
            .filter_map(|a| {
                a.message.as_ref().map(|m| {
                    let is_me = m.author.to_uppercase() == "USER";
                    Message {
                        id: a.name.clone(),
                        chat_id: chat_id.clone(),
                        sender_id: m.author.clone(),
                        content: MessageContent::Text(m.text.clone()),
                        timestamp: DateTime::parse_from_rfc3339(&a.create_time)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        is_from_me: is_me,
                        status: MessageStatus::Sent,
                    }
                })
            })
            .collect())
    }

    async fn send_message(
        &mut self,
        chat_id: &ChatId,
        content: MessageContent,
    ) -> AdapterResult<Message> {
        let text = match content {
            MessageContent::Text(t) => t,
            _ => return Err("Unsupported message type".into()),
        };

        let inner = self.inner.lock().await;
        inner
            .client
            .send_message(chat_id, &text)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Message {
            id: format!("temp_{}", Utc::now().timestamp()),
            chat_id: chat_id.clone(),
            sender_id: "user".to_string(),
            content: MessageContent::Text(text),
            timestamp: Utc::now(),
            is_from_me: true,
            status: MessageStatus::Sent,
        })
    }

    async fn mark_as_read(
        &mut self,
        _chat_id: &ChatId,
        _message_id: &MessageId,
    ) -> AdapterResult<()> {
        Ok(())
    }

    async fn get_contact(&self, _contact_id: &ContactId) -> AdapterResult<Contact> {
        Ok(Contact {
            id: "jules".to_string(),
            name: "Jules AI".to_string(),
            phone: None,
            avatar_url: None,
            status: Some("AI Coding Agent".to_string()),
            is_online: true,
        })
    }

    async fn get_contacts(&self) -> AdapterResult<Vec<Contact>> {
        Ok(vec![Contact {
            id: "jules".to_string(),
            name: "Jules AI".to_string(),
            phone: None,
            avatar_url: None,
            status: Some("AI Coding Agent".to_string()),
            is_online: true,
        }])
    }

    async fn subscribe_to_messages(
        &mut self,
    ) -> AdapterResult<tokio::sync::mpsc::Receiver<Message>> {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let inner_clone = self.inner.clone();

        tokio::spawn(async move {
            let mut last_processed_names: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let (client, session_ids) = {
                    let inner = inner_clone.lock().await;
                    let ids = if let Some(ref id) = inner.session_id {
                        vec![id.clone()]
                    } else if !inner.project_sessions.is_empty() {
                        inner
                            .project_sessions
                            .iter()
                            .map(|(id, _)| id.clone())
                            .collect()
                    } else {
                        // In global mode, poll all active sessions
                        match inner.client.list_sessions().await {
                            Ok(sessions) => {
                                sessions.into_iter().map(|s| s.id().to_string()).collect()
                            }
                            Err(_) => vec![],
                        }
                    };
                    (inner.client.clone(), ids)
                };

                for session_id in session_ids {
                    let last_name = last_processed_names.get(&session_id).cloned();
                    let result = if let Some(ref name) = last_name {
                        client.get_activities_after(&session_id, name, 10).await
                    } else {
                        client.get_activities(&session_id, 1).await
                    };

                    if let Ok(activities) = result {
                        for a in activities {
                            // If this was the first fetch for this session, just record the latest name
                            if last_name.is_none() {
                                last_processed_names.insert(session_id.clone(), a.name.clone());
                                continue;
                            }

                            if let Some(m) = a.message.as_ref() {
                                let is_me = m.author.to_uppercase() == "USER";
                                let msg = Message {
                                    id: a.name.clone(),
                                    chat_id: session_id.clone(),
                                    sender_id: m.author.clone(),
                                    content: MessageContent::Text(m.text.clone()),
                                    timestamp: DateTime::parse_from_rfc3339(&a.create_time)
                                        .map(|d| d.with_timezone(&Utc))
                                        .unwrap_or_else(|_| Utc::now()),
                                    is_from_me: is_me,
                                    status: MessageStatus::Sent,
                                };
                                if tx.send(msg).await.is_err() {
                                    return; // Channel closed
                                }
                            }
                            last_processed_names.insert(session_id.clone(), a.name.clone());
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn search(&self, _query: &str) -> AdapterResult<Vec<Chat>> {
        Ok(vec![])
    }

    fn requires_setup(&self) -> bool {
        let inner = match self.inner.try_lock() {
            Ok(i) => i,
            Err(_) => return false,
        };
        inner.client.api_key().is_empty()
    }

    async fn setup(&mut self, data: &str) -> AdapterResult<()> {
        let mut inner = self.inner.lock().await;
        crate::config::set_api_key(data).map_err(|e| e.to_string())?;
        inner.client = crate::api::JulesClient::new(data);
        Ok(())
    }
}
