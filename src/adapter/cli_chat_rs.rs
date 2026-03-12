//! cli-chat-rs MessagingAdapter blueprint for julesctl.
//! See: https://github.com/rvoidex7/cli-chat-rs/blob/main/ADAPTER_GUIDE.md
//!
//! Uncomment and adjust method signatures to match your cli-chat-rs version.

use crate::api::JulesClient;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

struct Inner {
    client: JulesClient,
    session_id: String,
    _last_activity_name: Option<String>,
}

pub struct JulesAdapter {
    inner: Arc<Mutex<Inner>>,
}

impl JulesAdapter {
    pub fn new(api_key: &str, session_id: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                client: JulesClient::new(api_key),
                session_id: session_id.to_string(),
                _last_activity_name: None,
            })),
        }
    }
}

use async_trait::async_trait;
use cli_chat_rs::{
    AdapterResult, Chat, ChatId, ConnectionStatus, Contact, ContactId, Message,
    MessageContent, MessageId, MessageStatus, MessagingAdapter,
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
        Ok(vec![Chat {
            id: inner.session_id.clone(),
            name: "Jules Session".to_string(),
            is_group: false,
            participants: vec!["jules".to_string()],
            last_message: None,
            unread_count: 0,
        }])
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
            let mut last_processed_name = None;
            
            // First pass to find the latest activity name
            {
                let inner = inner_clone.lock().await;
                if let Ok(activities) = inner.client.get_activities(&inner.session_id, 1).await {
                    last_processed_name = activities.last().map(|a| a.name.clone());
                }
            }

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                
                let (client, session_id) = {
                    let inner = inner_clone.lock().await;
                    (inner.client.clone(), inner.session_id.clone())
                };

                let result = if let Some(ref last_name) = last_processed_name {
                    client.get_activities_after(&session_id, last_name, 10).await
                } else {
                    client.get_activities(&session_id, 10).await
                };

                if let Ok(activities) = result {
                    for a in activities {
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
                        last_processed_name = Some(a.name.clone());
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn search(&self, _query: &str) -> AdapterResult<Vec<Chat>> {
        Ok(vec![])
    }
}
