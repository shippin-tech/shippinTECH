

use std::{path::PathBuf, time::SystemTime};

use crate::config::telemetry::{LogTracer, RotationStrategy};

use mail_parser::DateTime;
use tokio::{
    fs::{File, OpenOptions},
    io::BufWriter,
};
use trc::{TelemetryEvent, ipc::subscriber::SubscriberBuilder, serializers::text::FmtWriter};

pub(crate) fn spawn_log_tracer(builder: SubscriberBuilder, settings: LogTracer) {
    let (_, mut rx) = builder.register();
    tokio::spawn(async move {
        if let Some(writer) = settings.build_writer().await {
            let mut buf = FmtWriter::new(writer)
                .with_ansi(settings.ansi)
                .with_multiline(settings.multiline);
            let mut roatation_timestamp = settings.next_rotation();

            while let Some(events) = rx.recv().await {
                for event in events {
                    // Check if we need to rotate the log file
                    if roatation_timestamp != 0 && event.inner.timestamp > roatation_timestamp {
                        if let Err(err) = buf.flush().await {
                            trc::event!(
                                Telemetry(TelemetryEvent::LogError),
                                Reason = err.to_string(),
                                Details = "Failed to flush log buffer"
                            );
                        }

                        if let Some(writer) = settings.build_writer().await {
                            buf.update_writer(writer);
                            roatation_timestamp = settings.next_rotation();
                        } else {
                            return;
                        };
                    }

                    if let Err(err) = buf.write(&event).await {
                        trc::event!(
                            Telemetry(TelemetryEvent::LogError),
                            Reason = err.to_string(),
                            Details = "Failed to write event to log"
                        );
                        return;
                    }
                }

                if let Err(err) = buf.flush().await {
                    trc::event!(
                        Telemetry(TelemetryEvent::LogError),
                        Reason = err.to_string(),
                        Details = "Failed to flush log buffer"
                    );
                }
            }
        }
    });
}

impl LogTracer {
    pub async fn build_writer(&self) -> Option<BufWriter<File>> {
        if let Err(err) = tokio::fs::create_dir_all(&self.path).await {
            trc::event!(
                Telemetry(TelemetryEvent::LogError),
                Details = "Failed to create log directory",
                Path = self.path.clone(),
                Reason = err.to_string(),
            );
            return None;
        }
        let now = DateTime::from_timestamp(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()) as i64,
        );
        let file_name = match self.rotate {
            RotationStrategy::Daily => {
                format!(
                    "{}.{:04}-{:02}-{:02}",
                    self.prefix, now.year, now.month, now.day
                )
            }
            RotationStrategy::Hourly => {
                format!(
                    "{}.{:04}-{:02}-{:02}T{:02}",
                    self.prefix, now.year, now.month, now.day, now.hour
                )
            }
            RotationStrategy::Minutely => {
                format!(
                    "{}.{:04}-{:02}-{:02}T{:02}:{:02}",
                    self.prefix, now.year, now.month, now.day, now.hour, now.minute
                )
            }
            RotationStrategy::Never => self.prefix.clone(),
        };
        let path = PathBuf::from(&self.path).join(file_name);

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
        {
            Ok(writer) => Some(BufWriter::new(writer)),
            Err(err) => {
                trc::event!(
                    Telemetry(TelemetryEvent::LogError),
                    Details = "Failed to create log file",
                    Path = path.to_string_lossy().into_owned(),
                    Reason = err.to_string(),
                );
                None
            }
        }
    }

    pub fn next_rotation(&self) -> u64 {
        let mut now = DateTime::from_timestamp(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()) as i64,
        );

        now.second = 0;

        match self.rotate {
            RotationStrategy::Daily => {
                now.hour = 0;
                now.minute = 0;
                now.to_timestamp() as u64 + 86400
            }
            RotationStrategy::Hourly => {
                now.minute = 0;
                now.to_timestamp() as u64 + 3600
            }
            RotationStrategy::Minutely => now.to_timestamp() as u64 + 60,
            RotationStrategy::Never => 0,
        }
    }
}

#[cfg(test)]
mod install_defects_tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn missing_log_directories_are_created_and_existing_logs_are_appended() {
        let root = std::env::temp_dir().join(format!(
            "shippin-log-test-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let path = root.join("nested/logs");
        let mut settings = LogTracer {
            path: path.to_string_lossy().into_owned(),
            prefix: "server.log".into(),
            rotate: RotationStrategy::Never,
            ansi: false,
            multiline: false,
        };
        for line in [b"first\n", b"again\n"] {
            let mut writer = settings.build_writer().await.expect("writable directory");
            writer.write_all(line).await.unwrap();
            writer.flush().await.unwrap();
        }
        let file = path.join("server.log");
        assert_eq!(tokio::fs::read(&file).await.unwrap(), b"first\nagain\n");
        settings.path = file.join("not-a-directory").to_string_lossy().into_owned();
        assert!(settings.build_writer().await.is_none());
        assert_eq!(tokio::fs::read(&file).await.unwrap(), b"first\nagain\n");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
