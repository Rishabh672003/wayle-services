//! Fallback backend for systems running auto-cpufreq instead of power-profiles-daemon.
//!
//! auto-cpufreq has no D-Bus API: the forced governor lives in a root-owned pickle
//! file and is changed via `pkexec auto-cpufreq --force=<mode>`.

use std::{path::Path, sync::Weak, time::Duration};

use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::{
    core::PowerProfiles,
    error::Error,
    types::profile::{PowerProfile, Profile},
};

const BINARY: &str = "/usr/bin/auto-cpufreq";
const OVERRIDE_FILE: &str = "/opt/auto-cpufreq/override.pickle";
/// Present only while the systemd unit is active. It is a dangling symlink, so check
/// it with `symlink_metadata` rather than `exists()`.
const UNIT_MARKER: &str = "/run/systemd/units/invocation:auto-cpufreq.service";
const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub(crate) fn is_running() -> bool {
    Path::new(BINARY).exists() && std::fs::symlink_metadata(UNIT_MARKER).is_ok()
}

pub(crate) fn profiles() -> Vec<Profile> {
    [
        PowerProfile::PowerSaver,
        PowerProfile::Balanced,
        PowerProfile::Performance,
    ]
    .into_iter()
    .map(|profile| Profile {
        driver: String::from("auto-cpufreq"),
        profile,
    })
    .collect()
}

/// No override file means auto-cpufreq is in automatic mode, shown as Balanced.
pub(crate) fn read_profile() -> PowerProfile {
    std::fs::read(OVERRIDE_FILE)
        .map(|bytes| parse_override(&bytes))
        .unwrap_or(PowerProfile::Balanced)
}

/// The pickle holds a single short string, so a substring match is enough.
fn parse_override(bytes: &[u8]) -> PowerProfile {
    let contains = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
    if contains(b"performance") {
        PowerProfile::Performance
    } else if contains(b"powersave") {
        PowerProfile::PowerSaver
    } else {
        PowerProfile::Balanced
    }
}

pub(crate) async fn set_profile(profile: PowerProfile) -> Result<(), Error> {
    let mode = match profile {
        PowerProfile::PowerSaver => "powersave",
        PowerProfile::Performance => "performance",
        PowerProfile::Balanced | PowerProfile::Unknown => "reset",
    };

    let output = Command::new("pkexec")
        .arg(BINARY)
        .arg(format!("--force={mode}"))
        .output()
        .await
        .map_err(|err| Error::CommandFailed(format!("cannot run pkexec: {err}")))?;

    if !output.status.success() {
        return Err(Error::CommandFailed(format!(
            "auto-cpufreq --force={mode} exited with {}",
            output.status
        )));
    }
    Ok(())
}

// ponytail: polls a tiny file every 2s; switch to the `notify` crate if this ever shows up in profiles.
pub(crate) fn spawn_watch(weak: Weak<PowerProfiles>, cancel_token: CancellationToken) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    debug!("auto-cpufreq override watch cancelled");
                    return;
                }
                _ = interval.tick() => {
                    let Some(power_profiles) = weak.upgrade() else { return };
                    power_profiles.active_profile.set(read_profile());
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pickled_overrides() {
        // pickle.dumps('performance') / pickle.dumps('powersave')
        let perf = b"\x80\x05\x95\x0f\x00\x00\x00\x00\x00\x00\x00\x8c\x0bperformance\x94.";
        let save = b"\x80\x05\x95\x0d\x00\x00\x00\x00\x00\x00\x00\x8c\x09powersave\x94.";
        assert_eq!(parse_override(perf), PowerProfile::Performance);
        assert_eq!(parse_override(save), PowerProfile::PowerSaver);
        assert_eq!(parse_override(b""), PowerProfile::Balanced);
    }
}
