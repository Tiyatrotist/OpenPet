//! # OpenPet Plugin Host
//!
//! Sandboxed execution environment, capability enforcement, and fault-isolation runtime.

use openpet_plugin_api::{PetCommand, PluginError};
use openpet_types::PluginCapability;
use std::collections::HashSet;
use tracing::{error, info};

pub const DEFAULT_PLUGIN_FUEL_BUDGET: u64 = 1_000_000;

pub struct PluginInstance {
    pub id: String,
    pub name: String,
    pub granted_capabilities: HashSet<PluginCapability>,
    pub fuel_remaining: u64,
    pub is_enabled: bool,
    pub is_faulted: bool,
}

impl PluginInstance {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            // Default plugin capabilities: strictly NONE!
            granted_capabilities: HashSet::new(),
            fuel_remaining: DEFAULT_PLUGIN_FUEL_BUDGET,
            is_enabled: true,
            is_faulted: false,
        }
    }

    pub fn grant_capability(&mut self, cap: PluginCapability) {
        self.granted_capabilities.insert(cap);
    }

    /// Safely processes a PetCommand with capability validation and fuel deduction.
    ///
    /// If plugin faults or exceeds fuel, disables plugin and isolates failure from host runtime.
    pub fn submit_command(&mut self, command: PetCommand) -> Result<(), PluginError> {
        if !self.is_enabled || self.is_faulted {
            return Err(PluginError::Execution(
                "Plugin is disabled or faulted".into(),
            ));
        }

        // Check fuel
        if self.fuel_remaining < 100 {
            self.is_faulted = true;
            self.is_enabled = false;
            error!(
                "Plugin '{}' exceeded fuel quota. Disabled to protect host.",
                self.name
            );
            return Err(PluginError::OutOfFuel);
        }
        self.fuel_remaining -= 100;

        // Verify capability permissions:
        match &command {
            PetCommand::PlayAnimation { .. } | PetCommand::ShowEmote(_) => {
                if !self
                    .granted_capabilities
                    .contains(&PluginCapability::PetCommand)
                {
                    return Err(PluginError::CapabilityDenied("pet.command".into()));
                }
            }
            PetCommand::DispatchNotification { .. } => {
                if !self
                    .granted_capabilities
                    .contains(&PluginCapability::Notifications)
                {
                    return Err(PluginError::CapabilityDenied("notifications".into()));
                }
            }
            PetCommand::TemporaryMoodModifier { .. } | PetCommand::RequestInteraction { .. } => {
                if !self
                    .granted_capabilities
                    .contains(&PluginCapability::PetCommand)
                {
                    return Err(PluginError::CapabilityDenied("pet.command".into()));
                }
            }
        }

        info!("Plugin '{}' executed command: {:?}", self.name, command);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpet_types::BehaviorType;

    #[test]
    fn test_least_privilege_default_denial() {
        let mut plugin = PluginInstance::new("pomodoro", "Pomodoro Timer");
        // By default, NO capabilities granted!

        let cmd = PetCommand::PlayAnimation {
            behavior: BehaviorType::Happy,
            loops: 1,
        };

        let result = plugin.submit_command(cmd);
        assert!(matches!(result, Err(PluginError::CapabilityDenied(_))));
    }

    #[test]
    fn test_granted_capability_succeeds() {
        let mut plugin = PluginInstance::new("pomodoro", "Pomodoro Timer");
        plugin.grant_capability(PluginCapability::PetCommand);

        let cmd = PetCommand::PlayAnimation {
            behavior: BehaviorType::Happy,
            loops: 1,
        };

        assert!(plugin.submit_command(cmd).is_ok());
    }

    #[test]
    fn test_fuel_exhaustion_isolates_fault() {
        let mut plugin = PluginInstance::new("loop_bug", "Buggy Loop");
        plugin.grant_capability(PluginCapability::PetCommand);
        plugin.fuel_remaining = 50; // below 100 fuel

        let cmd = PetCommand::PlayAnimation {
            behavior: BehaviorType::Happy,
            loops: 1,
        };

        let res = plugin.submit_command(cmd);
        assert_eq!(res, Err(PluginError::OutOfFuel));
        assert!(plugin.is_faulted);
        assert!(!plugin.is_enabled);
    }
}
