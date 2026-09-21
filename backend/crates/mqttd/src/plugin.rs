//! Plugin registration module for the `mqttd` library.

use anyhow::Result;
use rmqtt::context::ServerContext;

// Built-in plugin modules
use crate::plugins::auth;
#[cfg(feature = "message-storage")]
use crate::plugins::message;
use crate::plugins::restapi;
use crate::plugins::retainer;
use crate::plugins::session;
use crate::plugins::transfer;
#[cfg(feature = "webhook")]
use crate::plugins::webhook;

/// Register built-in plugins under their configuration file names.
/// Only plugins listed in `default_startups` are started immediately.
pub async fn registers(scx: &ServerContext, default_startups: Vec<String>) -> Result<()> {
    registers_with_mode(scx, default_startups, false).await
}

pub(crate) async fn registers_with_mode(
    scx: &ServerContext,
    default_startups: Vec<String>,
    embedded: bool,
) -> Result<()> {
    // Authentication plugin
    auth::register_named(
        scx,
        "auth",
        default_startups.iter().any(|name| name == "auth"),
        false,
    )
    .await?;
    // Message storage plugin (optional)
    #[cfg(feature = "message-storage")]
    {
        message::register_named(
            scx,
            "message",
            default_startups.iter().any(|name| name == "message"),
            false,
        )
        .await?;
    }
    // REST API plugin
    if !embedded {
        restapi::register_named(
            scx,
            "restapi",
            default_startups.iter().any(|name| name == "restapi"),
            false,
        )
        .await?;
    }
    // Retainer plugin
    retainer::register_named(
        scx,
        "retainer",
        default_startups.iter().any(|name| name == "retainer"),
        false,
    )
    .await?;
    // Session plugin
    session::register_named(
        scx,
        "session",
        default_startups.iter().any(|name| name == "session"),
        false,
    )
    .await?;
    // Transfer plugin
    transfer::register_named(
        scx,
        "transfer",
        default_startups.iter().any(|name| name == "transfer"),
        false,
    )
    .await?;
    // Webhook plugin (optional)
    #[cfg(feature = "webhook")]
    {
        webhook::register_named(
            scx,
            "webhook",
            default_startups.iter().any(|name| name == "webhook"),
            false,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn plugins_have_distinct_names_and_remain_stopped_when_not_selected() -> Result<()> {
        let scx = ServerContext::new().busy_check_enable(false).build().await;
        registers(&scx, vec![]).await?;
        for name in ["auth", "restapi", "retainer", "session", "transfer"] {
            let entry = scx
                .plugins
                .get(name)
                .expect("plugin must have its own name");
            assert!(!entry.inited());
            assert!(!entry.active());
        }
        #[cfg(feature = "message-storage")]
        assert!(scx.plugins.get("message").is_some());
        #[cfg(feature = "webhook")]
        assert!(scx.plugins.get("webhook").is_some());
        assert!(scx.plugins.get("mqttd").is_none());
        Ok(())
    }

    #[tokio::test]
    async fn selected_auth_plugin_loads_its_named_config_and_starts() -> Result<()> {
        let scx = ServerContext::new()
            .busy_check_enable(false)
            .plugins_config_map_add("auth", "priority = 10")
            .build()
            .await;
        registers(&scx, vec!["auth".to_owned()]).await?;
        assert!(scx.plugins.is_active("auth"));
        assert!(!scx.plugins.is_active("session"));
        assert!(!scx.plugins.is_active("restapi"));
        assert!(scx.plugins.get("mqttd").is_none());
        assert!(scx.plugins.stop("auth").await?);
        Ok(())
    }
}
