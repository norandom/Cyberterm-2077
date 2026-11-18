use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use channel_versions::{Changelog, VersionInfo};
use warpui::{AppContext, Entity, ModelContext, SingletonEntity, ViewContext};

use crate::server::server_api::ServerApi;
use crate::workspace::Workspace;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum AutoupdateStage {
    #[default]
    NoUpdateAvailable,
    CheckingForUpdate,
    DownloadingUpdate,
    UnableToUpdateToNewVersion {
        new_version: VersionInfo,
    },
    UpdateReady {
        new_version: VersionInfo,
        update_id: String,
    },
    Updating {
        new_version: VersionInfo,
        update_id: String,
    },
    UnableToLaunchNewVersion {
        new_version: VersionInfo,
    },
    UpdatedPendingRestart {
        new_version: VersionInfo,
    },
}

impl AutoupdateStage {
    pub fn ready_for_update(&self) -> bool {
        matches!(
            self,
            Self::UpdateReady { .. } | Self::UpdatedPendingRestart { .. }
        )
    }

    pub fn available_new_version(&self) -> Option<&VersionInfo> {
        match self {
            Self::UpdateReady { new_version, .. }
            | Self::Updating { new_version, .. }
            | Self::UnableToUpdateToNewVersion { new_version }
            | Self::UnableToLaunchNewVersion { new_version }
            | Self::UpdatedPendingRestart { new_version } => Some(new_version),
            _ => None,
        }
    }
}

pub struct AutoupdateState {
    stage: AutoupdateStage,
    polling: bool,
    last_check: Option<Instant>,
    last_error: Option<String>,
    installed_exe: Option<std::path::PathBuf>,
    on_shutdown_ready: Option<Box<dyn FnOnce(&mut AppContext) + Send>>,
    #[cfg(windows)]
    windows_helper: Option<(std::path::PathBuf, String)>,
}

impl AutoupdateState {
    pub fn new(_server_api: Arc<ServerApi>) -> Self {
        Self {
            stage: AutoupdateStage::NoUpdateAvailable,
            polling: false,
            last_check: None,
            last_error: None,
            installed_exe: None,
            on_shutdown_ready: None,
            #[cfg(windows)]
            windows_helper: None,
        }
    }

    pub fn register(ctx: &mut AppContext, server_api: Arc<ServerApi>) {
        ctx.add_singleton_model(|_| Self::new(server_api));
        check_and_report_update_errors(ctx);
    }

    pub fn start_polling(&mut self, ctx: &mut ModelContext<Self>) {
        if self.polling {
            return;
        }
        self.polling = true;
        // Keep a failed helper's diagnostic visible until the user retries.
        if self.last_error.is_some() {
            self.last_check = Some(Instant::now());
        }
        self.poll(ctx);
    }

    fn poll(&mut self, ctx: &mut ModelContext<Self>) {
        self.maybe_daily_check_for_update(ctx);
        ctx.spawn(
            async {
                warpui::r#async::Timer::after(Duration::from_secs(3600)).await;
            },
            |me, _, ctx| me.poll(ctx),
        );
    }

    pub fn manually_check_for_update(&mut self, ctx: &mut ModelContext<Self>) {
        self.check(RequestType::ManualCheck, ctx);
    }

    pub fn maybe_daily_check_for_update(&mut self, ctx: &mut ModelContext<Self>) {
        if self
            .last_check
            .is_none_or(|time| time.elapsed() >= Duration::from_secs(86400))
        {
            self.check(RequestType::DailyCheck, ctx);
        }
    }

    fn check(&mut self, request_type: RequestType, ctx: &mut ModelContext<Self>) {
        if matches!(
            self.stage,
            AutoupdateStage::CheckingForUpdate
                | AutoupdateStage::Updating { .. }
                | AutoupdateStage::UpdatedPendingRestart { .. }
        ) {
            return;
        }
        self.last_check = Some(Instant::now());
        self.last_error = None;
        self.stage = AutoupdateStage::CheckingForUpdate;
        ctx.notify();
        ctx.spawn(background(warpeer_update::check), move |me, result, ctx| {
            let result = result.map(|version| match version {
                Some(tag) => {
                    let new_version = VersionInfo::new(tag.clone());
                    if warpeer_update::package_managed() {
                        me.stage = AutoupdateStage::UnableToUpdateToNewVersion {
                            new_version: new_version.clone(),
                        };
                        return UpdateReady::CanDownload {
                            new_version,
                            update_id: tag,
                        };
                    }
                    me.stage = AutoupdateStage::UpdateReady {
                        new_version: new_version.clone(),
                        update_id: tag.clone(),
                    };
                    ctx.emit(AutoupdateStateEvent::UpdateAvailable);
                    UpdateReady::Yes {
                        new_version,
                        update_id: tag,
                    }
                }
                None => {
                    me.stage = AutoupdateStage::NoUpdateAvailable;
                    UpdateReady::No
                }
            });
            if let Err(err) = &result {
                me.last_error = Some(format!("{err:#}"));
                me.stage = AutoupdateStage::NoUpdateAvailable;
                log::warn!("Warpeer update check failed: {err:#}");
            }
            ctx.emit(AutoupdateStateEvent::CheckComplete {
                result: Box::new(result),
                request_type,
            });
            ctx.notify();
        });
    }

    fn install(&mut self, ctx: &mut ModelContext<Self>) {
        let AutoupdateStage::UpdateReady {
            new_version,
            update_id,
        } = self.stage.clone()
        else {
            return;
        };
        let tag = update_id.clone();
        // Capture this before the updater moves the running executable aside.
        let exe = std::env::current_exe();
        self.stage = AutoupdateStage::Updating {
            new_version: new_version.clone(),
            update_id,
        };
        self.last_error = None;
        ctx.notify();
        #[cfg(windows)]
        {
            match warpeer_update::prepare_windows_helper().and_then(|helper| Ok((helper, exe?))) {
                Ok((helper, exe)) => {
                    self.windows_helper = Some((helper, tag.clone()));
                    self.installed_exe = Some(exe);
                    self.stage = AutoupdateStage::UpdateReady {
                        new_version,
                        update_id: tag,
                    };
                    request_restart(ctx);
                }
                Err(err) => {
                    self.last_error = Some(format!("{err:#}"));
                    self.stage = AutoupdateStage::UnableToUpdateToNewVersion { new_version };
                    ctx.notify();
                }
            }
        }
        #[cfg(not(windows))]
        ctx.spawn(
            background(move || {
                let exe = exe?;
                warpeer_update::install(&tag)?;
                Ok(exe)
            }),
            move |me, result, ctx| {
                let succeeded = result.is_ok();
                match result {
                    Ok(exe) => {
                        me.installed_exe = Some(exe);
                        me.stage = AutoupdateStage::UpdatedPendingRestart { new_version };
                    }
                    Err(err) => {
                        log::error!("Warpeer update failed: {err:#}");
                        me.last_error = Some(format!("{err:#}"));
                        me.stage = AutoupdateStage::UnableToUpdateToNewVersion { new_version };
                    }
                }
                ctx.notify();
                if let Some(callback) = me.on_shutdown_ready.take() {
                    callback(ctx);
                } else if succeeded {
                    request_restart(ctx);
                }
            },
        );
    }
}

async fn background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    let (send, recv) = futures::channel::oneshot::channel();
    std::thread::Builder::new()
        .name("warpeer-update".into())
        .spawn(move || {
            let _ = send.send(work());
        })?;
    recv.await.map_err(|_| anyhow!("Update worker stopped"))?
}

pub fn update_error(app: &AppContext) -> Option<&str> {
    AutoupdateState::as_ref(app).last_error.as_deref()
}

pub fn has_checked(app: &AppContext) -> bool {
    AutoupdateState::as_ref(app).last_check.is_some()
}

impl Entity for AutoupdateState {
    type Event = AutoupdateStateEvent;
}

impl SingletonEntity for AutoupdateState {}

pub enum AutoupdateStateEvent {
    CheckComplete {
        result: Box<Result<UpdateReady>>,
        request_type: RequestType,
    },
    UpdateAvailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateReady {
    Yes {
        new_version: VersionInfo,
        update_id: String,
    },
    CanDownload {
        new_version: VersionInfo,
        update_id: String,
    },
    No,
}

pub enum ReadyForRelaunch {
    Yes,
    No,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestType {
    ManualCheck,
    Poll,
    DailyCheck,
}

pub fn get_update_state(app: &AppContext) -> AutoupdateStage {
    AutoupdateState::as_ref(app).stage.clone()
}

pub fn apply_update(
    _initiating_workspace: &mut Workspace,
    ctx: &mut ViewContext<Workspace>,
) -> Result<ReadyForRelaunch> {
    initiate_relaunch_for_update(ctx);
    Ok(ReadyForRelaunch::No)
}

pub fn initiate_relaunch_for_update(app: &mut AppContext) {
    #[cfg(windows)]
    if AutoupdateState::as_ref(app).windows_helper.is_some() {
        request_restart(app);
        return;
    }
    if matches!(
        get_update_state(app),
        AutoupdateStage::UpdatedPendingRestart { .. }
    ) {
        request_restart(app);
    } else {
        AutoupdateState::handle(app).update(app, |me, ctx| me.install(ctx));
    }
}

fn request_restart(app: &mut AppContext) {
    RelaunchModel::handle(app).update(app, |me, ctx| {
        me.requested = true;
        ctx.notify();
    });
    app.terminate_app(warpui::platform::TerminationMode::Cancellable, None);
}

pub fn apply_pending_update<F>(app: &mut AppContext, on_update_complete: F) -> bool
where
    F: FnOnce(&mut AppContext) + Send + 'static,
{
    AutoupdateState::handle(app).update(app, |me, _| {
        if matches!(me.stage, AutoupdateStage::Updating { .. }) {
            // Normal quit must wait for the worker to finish replacing the app.
            me.on_shutdown_ready = Some(Box::new(on_update_complete));
            true
        } else {
            false
        }
    })
}

pub fn cancel_relaunch(app: &mut AppContext) {
    RelaunchModel::handle(app).update(app, |me, ctx| {
        me.requested = false;
        ctx.notify();
    });
    #[cfg(windows)]
    AutoupdateState::handle(app).update(app, |me, _| {
        if let Some((helper, _)) = me.windows_helper.take() {
            let _ = std::fs::remove_file(&helper);
            if let Some(parent) = helper.parent() {
                let _ = std::fs::remove_dir(parent);
            }
        }
    });
}

pub fn spawn_child_if_necessary(app: &mut AppContext) {
    if !RelaunchModel::as_ref(app).requested {
        return;
    }
    if let Some(exe) = &AutoupdateState::as_ref(app).installed_exe {
        #[cfg(windows)]
        if let Some((helper, tag)) = &AutoupdateState::as_ref(app).windows_helper {
            if let Err(err) = std::process::Command::new(helper)
                .arg(std::process::id().to_string())
                .arg(tag)
                .arg(exe)
                .spawn()
            {
                log::error!("Unable to start update helper: {err}");
                let _ = std::process::Command::new(exe).spawn();
            }
            return;
        }
        if let Err(err) = std::process::Command::new(exe).spawn() {
            log::error!("Unable to relaunch Warpeer: {err}");
        }
    }
}

pub fn manually_download_new_version(ctx: &mut AppContext) {
    ctx.open_url(warpeer_update::RELEASES_URL);
}

pub(crate) fn check_and_report_update_errors(ctx: &mut AppContext) {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent() {
            let path = root.join(".warpeer-update-error");
            if let Ok(message) = std::fs::read_to_string(&path) {
                log::error!("Previous update failed: {message}");
                AutoupdateState::handle(ctx).update(ctx, |me, ctx| {
                    me.last_error = Some(message);
                    ctx.notify();
                });
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

pub fn remove_old_executable() -> Result<()> {
    Ok(())
}

#[derive(Clone, Copy, Default)]
pub struct RelaunchModel {
    requested: bool,
}

impl RelaunchModel {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Entity for RelaunchModel {
    type Event = ();
}

impl SingletonEntity for RelaunchModel {}

pub fn is_incoming_version_past_current(version: Option<&str>) -> bool {
    version.is_some_and(|v| warpeer_update::newer(v, warpeer_update::BUILD_TAG).unwrap_or(false))
}

pub async fn get_current_changelog(_server_api: Arc<ServerApi>) -> Result<Option<Changelog>> {
    Ok(None)
}
