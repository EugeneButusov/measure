use std::{env, thread, time};
use std::sync::mpsc::channel;
use log::{info};
use simple_logger::SimpleLogger;
use pullse::ledger::{PullseLedger};
use pullse::gathering::get_gatherers;
use pullse::exposing::get_exposers;
use pullse::settings::Settings;

fn main() {
    SimpleLogger::new().init().unwrap();

    let settings = if let Ok(custom_config_path) = env::var("CONFIG_PATH") {
        Settings::new_from_custom_config(custom_config_path)
    } else {
        Settings::new_default()
    }.expect("Config cannot be read as it's corrupted");

    info!("Bootstrapping started...");
    let mut ledger = PullseLedger::new();

    let pullers = get_gatherers(&settings.gatherers);
    for puller in &pullers {
        let pulled_data = puller.gather();
        for entry in pulled_data {
            ledger.insert(entry);
        }
    }

    let consumers = get_exposers(&ledger, &settings.exposers);
    info!("Bootstrapping completed!");

    info!("Runloop initiated");

    let (tx, rx) = channel();
    let pull_thread = thread::spawn(move || {
        loop {
            // TODO: perform pull
            for puller in &pullers {
                let pulled_data = puller.gather();
                for entry in pulled_data {
                    tx.send(entry).unwrap(); // TODO: add proper error handling
                }
            }
            thread::sleep(time::Duration::from_millis(settings.common.pull_timeout));
        }
    });

    let publish_thread = thread::spawn(move || while let Ok(entry) = rx.recv() {
        ledger.insert(entry);
        for consumer in &consumers {
            consumer.consume(&ledger);
        }
    });

    pull_thread.join().unwrap();
    publish_thread.join().unwrap();
}
