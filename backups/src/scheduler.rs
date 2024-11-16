use std::sync::Arc;
use std::time::Duration;

use tracing::*;

use clokwerk::AsyncScheduler;
use clokwerk::Interval;
use clokwerk::Job;

use crate::backup::make_backup;
use crate::config::*;

pub async fn run_backup_tasks(config: Config) -> anyhow::Result<()> {
  for task_config in config.tasks.iter().cloned() {
    spawn_backup_task(task_config).await?;
  }

  Ok(())
}

pub async fn spawn_backup_task(config: BackupTaskConfig) -> anyhow::Result<()> {
  match config.on.trigger {
    BackupTrigger::Schedule { ref every, ref at } => {
      let mut intervals = parse_schedule(every)?.into_iter();

      let Some(first_interval) = intervals.next() else {
        anyhow::bail!("no intervals provided");
      };

      let mut scheduler = AsyncScheduler::new();
      let task = scheduler.every(first_interval);

      intervals.for_each(|interval| {
        task.and_every(interval);
      });

      if let Some(at) = at {
        task.at(at);
      }

      let config = Arc::new(config);

      task.forever().run(move || {
        let config = config.clone();
        async move {
          let span = info_span!(
            "backup",
            r#type = config.on.strategy.to_string(),
            src = config.source.display().to_string(),
            dst = config.destination.display().to_string()
          );

          let _guard = span.enter();
          let start = std::time::Instant::now();
          match make_backup(&config) {
            Ok(()) => info!("backup completed in {:?}", start.elapsed()),
            Err(e) => error!("backup failed after {:?}: {}", start.elapsed(), e),
          }
        }
      });

      tokio::spawn(async move {
        loop {
          scheduler.run_pending().await;
          tokio::time::sleep(Duration::from_secs(1)).await;
        }
      });
    }
  }

  Ok(())
}

fn parse_schedule(every: &Vec<String>) -> anyhow::Result<Vec<Interval>> {
  const UNITS: &[&str] = &[
    "day",
    "days",
    "hour",
    "hours",
    "minute",
    "minutes",
    "second",
    "seconds",
    "weekday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
  ];

  let mut intervals = Vec::new();

  for every in every {
    let (count, unit) = every.split_once(' ').unwrap_or(("1", every));
    let count = count.parse::<u32>()?;
    let unit = unit.to_lowercase();

    if !UNITS.contains(&unit.as_str()) {
      anyhow::bail!("invalid unit: {}, must be one of: {}", unit, UNITS.join(", "));
    }

    let interval = match unit.as_str() {
      "day" | "days" => Interval::Days(count),
      "hour" | "hours" => Interval::Hours(count),
      "minute" | "minutes" => Interval::Minutes(count),
      "second" | "seconds" => Interval::Seconds(count),
      "weekday" => Interval::Weekday,
      "monday" => Interval::Monday,
      "tuesday" => Interval::Tuesday,
      "wednesday" => Interval::Wednesday,
      "thursday" => Interval::Thursday,
      "friday" => Interval::Friday,
      "saturday" => Interval::Saturday,
      "sunday" => Interval::Sunday,
      _ => unreachable!(),
    };

    intervals.push(interval);
  }

  Ok(intervals)
}
