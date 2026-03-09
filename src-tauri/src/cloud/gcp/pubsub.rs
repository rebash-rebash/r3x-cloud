use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Pub/Sub topics in the project.
pub async fn scan_topics(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "PubSubTopic".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://pubsub.googleapis.com/v1/projects/{}/topics",
        provider.project_id
    );

    let resp = provider
        .client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        // Pub/Sub API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "PubSubTopic".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "PubSubTopic".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list Pub/Sub topics: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(topics) = data["topics"].as_array() {
        for topic in topics {
            if let Some(resource) = parse_topic(topic, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "PubSubTopic".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_topic(topic: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let full_name = topic["name"].as_str()?.to_string();
    // Topic name format: projects/{project}/topics/{topic_name}
    let short_name = full_name
        .rsplit('/')
        .next()
        .unwrap_or(&full_name)
        .to_string();

    let mut tags = HashMap::new();
    if let Some(labels) = topic["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "full_name": full_name,
        "kms_key_name": topic["kmsKeyName"],
        "schema_settings": topic["schemaSettings"],
        "message_retention_duration": topic["messageRetentionDuration"],
    });

    // Pub/Sub pricing: $40/TB published, idle topics cost $0
    let cost = estimate_topic_cost();

    Some(CloudResource {
        id: full_name.clone(),
        name: short_name,
        resource_type: ResourceType::PubSubTopic,
        provider: ProviderKind::Gcp,
        region: "global".to_string(),
        account_id: project_id.to_string(),
        status: "ACTIVE".to_string(),
        created_at: None,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Pub/Sub topics.
/// $40/TB published. Without usage data, idle topics cost ~$0.
fn estimate_topic_cost() -> Option<f64> {
    Some(0.0)
}

/// Scan all Pub/Sub subscriptions in the project.
pub async fn scan_subscriptions(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "PubSubSubscription".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://pubsub.googleapis.com/v1/projects/{}/subscriptions",
        provider.project_id
    );

    let resp = provider
        .client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        // Pub/Sub API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "PubSubSubscription".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "PubSubSubscription".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!(
            "Failed to list Pub/Sub subscriptions: {} {}",
            status,
            body
        ));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(subscriptions) = data["subscriptions"].as_array() {
        for subscription in subscriptions {
            if let Some(resource) = parse_subscription(subscription, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "PubSubSubscription".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_subscription(
    subscription: &serde_json::Value,
    project_id: &str,
) -> Option<CloudResource> {
    let full_name = subscription["name"].as_str()?.to_string();
    // Subscription name format: projects/{project}/subscriptions/{sub_name}
    let short_name = full_name
        .rsplit('/')
        .next()
        .unwrap_or(&full_name)
        .to_string();

    let topic = subscription["topic"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let ack_deadline_seconds = subscription["ackDeadlineSeconds"].as_u64().unwrap_or(10);
    let message_retention_duration = subscription["messageRetentionDuration"]
        .as_str()
        .unwrap_or("604800s")
        .to_string();

    let has_push_config = subscription["pushConfig"]["pushEndpoint"]
        .as_str()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    // Determine status: DETACHED if topic is "_deleted-topic_", otherwise ACTIVE
    let status = if topic.contains("_deleted-topic_") {
        "DETACHED".to_string()
    } else {
        "ACTIVE".to_string()
    };

    let mut tags = HashMap::new();
    if let Some(labels) = subscription["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "full_name": full_name,
        "topic": topic,
        "ack_deadline_seconds": ack_deadline_seconds,
        "message_retention_duration": message_retention_duration,
        "push_endpoint": subscription["pushConfig"]["pushEndpoint"],
        "has_push_config": has_push_config,
        "filter": subscription["filter"],
        "dead_letter_policy": subscription["deadLetterPolicy"],
        "retry_policy": subscription["retryPolicy"],
        "expiration_policy": subscription["expirationPolicy"],
    });

    // Pub/Sub pricing: $40/TB delivered + storage for retained messages.
    // Without usage data, estimate $0 for idle subscriptions.
    let cost = estimate_subscription_cost();

    Some(CloudResource {
        id: full_name.clone(),
        name: short_name,
        resource_type: ResourceType::PubSubSubscription,
        provider: ProviderKind::Gcp,
        region: "global".to_string(),
        account_id: project_id.to_string(),
        status,
        created_at: None,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Pub/Sub subscriptions.
/// $40/TB delivered, plus storage for retained messages.
/// Without usage data, idle subscriptions cost ~$0.
fn estimate_subscription_cost() -> Option<f64> {
    Some(0.0)
}
